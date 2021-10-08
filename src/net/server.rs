use std::{any::Any, collections::HashMap, future::Future, num::NonZeroU64, pin::Pin};

use alkahest::{Pack, Schema, Seq, Unpacked};
use arcana_time::TimeSpan;
use astral::{
    channel::tcp::TcpChannel,
    client_server::{ClientId, Event, MaybePlayerId, PlayerId, ServerSession},
};
use hecs::{Component, QueryOneError, World};
use scoped_arena::Scope;
use tokio::net::TcpListener;
use tracing::instrument;

use crate::{CommandQueue, Res, Spawner, SystemContext};

use super::{EntityMapper, NetId};

/// Component to signal that entity is owned by server and must be replicated to connected clients.
pub struct ServerOwned;

pub struct IdGen {
    next: NonZeroU64,
}

impl IdGen {
    pub fn new() -> Self {
        IdGen {
            next: NonZeroU64::new(1).unwrap(),
        }
    }

    pub fn gen_nid(&mut self) -> NetId {
        NetId(self.gen())
    }

    pub fn gen_pid(&mut self) -> PlayerId {
        PlayerId(self.gen())
    }

    pub fn gen(&mut self) -> NonZeroU64 {
        let id = self.next;
        let next = self
            .next
            .get()
            .checked_add(1)
            .expect("u64 increment overflow");

        self.next = NonZeroU64::new(next).unwrap();

        id
    }
}

pub trait ReplicaSetElem<'a> {
    type Component: Component;
    type Replica: Schema;
    type ReplicaPack: Pack<Self::Replica> + Copy + 'a;

    fn replicate(component: &'a Self::Component, scope: &'a Scope<'_>) -> Self::ReplicaPack;
}

pub trait ReplicaSet<'a> {
    type Replica: Schema;
    type ReplicaPack: Pack<Self::Replica> + Copy;

    fn replicate(
        id_gen: &mut IdGen,
        mapper: &mut EntityMapper,
        world: &'a mut World,
        scope: &'a Scope<'_>,
    ) -> &'a [Self::ReplicaPack];
}

impl<'a> ReplicaSet<'a> for () {
    type Replica = (NetId, MaybePlayerId);
    type ReplicaPack = (NetId, Option<PlayerId>);

    fn replicate(
        id_gen: &mut IdGen,
        mapper: &mut EntityMapper,
        world: &mut World,
        scope: &'a Scope<'_>,
    ) -> &'a [(NetId, Option<PlayerId>)] {
        let mut new_nids = Vec::new_in(scope);

        let mut vec = Vec::with_capacity_in(65536, scope);
        vec.extend(
            world
                .query_mut::<(Option<&NetId>, Option<&PlayerId>)>()
                .with::<ServerOwned>()
                .into_iter()
                .map(|(e, (nid_opt, pid_opt))| {
                    let nid = match nid_opt {
                        None => {
                            let nid = mapper.new_nid(id_gen, e);
                            new_nids.push((e, nid));
                            nid
                        }
                        Some(nid) => *nid,
                    };
                    (nid, pid_opt.copied())
                }),
        );

        for (e, nid) in new_nids {
            world.insert_one(e, nid);
        }

        vec.leak()
    }
}

macro_rules! replica_set_tuple {
    ($($a:ident),+) => {
        impl<'a, $($a),+> ReplicaSet<'a> for ($($a,)+)
        where
            $($a: ReplicaSetElem<'a>,)+
        {
            type Replica = (NetId, MaybePlayerId, $(Option<$a::Replica>),+);
            type ReplicaPack = (NetId, Option<PlayerId>, $(Option<$a::ReplicaPack>),+);

            fn replicate(
                id_gen: &mut IdGen,
                mapper: &mut EntityMapper,
                world: &'a mut World,
                scope: &'a Scope<'_>
            ) -> &'a[(NetId, Option<PlayerId>, $(Option<$a::ReplicaPack>),+)] {
                #![allow(non_snake_case)]

                let mut new_nids = Vec::new_in(scope);

                for (e, nid_opt) in world.query_mut::<Option<&NetId>>().with::<ServerOwned>() {
                    let nid = match nid_opt {
                        None => {
                            let nid = mapper.new_nid(id_gen, e);
                            new_nids.push((e, nid));
                            nid
                        },
                        Some(nid) => *nid,
                    };
                }

                for (e, nid) in new_nids {
                    world.insert_one(e, nid);
                }

                let mut vec = Vec::with_capacity_in(65536, scope);
                vec.extend(
                    world
                        .query_mut::<(&NetId, Option<&PlayerId>, $(Option<&$a::Component>),+)>()
                        .with::<ServerOwned>()
                        .into_iter()
                        .map(|(e, (&nid, pid_opt, $($a,)+))| {
                            (nid, pid_opt.copied(), $($a.map(|$a| $a::replicate($a, scope)),)+)
                        })
                );

                vec.leak()
            }
        }
    };
}

replica_set_tuple!(A);
replica_set_tuple!(A, B);
replica_set_tuple!(A, B, C);
replica_set_tuple!(A, B, C, D);

/// Data associated with spawned player.
/// This may be as simple as controlled entity.
/// Or any kind of data structure.
pub trait RemotePlayer: Send + Sync + 'static {
    type Command: Component;
    type Info: Schema;
    type Input: Schema;

    /// Verifies player info.
    /// On success spawns required entities.
    /// Returns `Self` connected to spawned entities.
    /// On error returns a reason.
    fn accept(
        info: Unpacked<'_, Self::Info>,
        pid: PlayerId,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
    ) -> eyre::Result<Self>
    where
        Self: Sized;

    fn replicate_input(
        &mut self,
        input: Unpacked<'_, Self::Input>,
        queue: &mut CommandQueue<Self::Command>,
        scope: &Scope<'_>,
    );
}

pub struct ServerSystem {
    session: ServerSession<TcpChannel, TcpListener>,
    players: HashMap<PlayerId, ConnectedPlayer<dyn Any + Send + Sync>>,
    id_gen: IdGen,
    mapper: EntityMapper,

    run_impl: for<'r> fn(
        &'r mut ServerSession<TcpChannel, TcpListener>,
        &'r mut IdGen,
        &'r mut EntityMapper,
        &'r mut HashMap<PlayerId, ConnectedPlayer<dyn Any + Send + Sync>>,
        SystemContext<'r>,
    ) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + 'r>>,
}

struct ConnectedPlayer<P: ?Sized> {
    cid: ClientId,
    player: Box<P>,
}

impl ServerSystem {
    pub fn new<P, R>(listener: TcpListener, step_delta: TimeSpan) -> Self
    where
        P: RemotePlayer,
        R: for<'a> ReplicaSet<'a>,
    {
        ServerSystem {
            session: ServerSession::new(listener, step_delta.as_nanos()),
            players: HashMap::new(),
            id_gen: IdGen::new(),
            mapper: EntityMapper::new(),
            run_impl: run_impl::<P, R>,
        }
    }
}

fn run_impl<'a, P, R>(
    session: &'a mut ServerSession<TcpChannel, TcpListener>,
    id_gen: &'a mut IdGen,
    mapper: &'a mut EntityMapper,
    players: &'a mut HashMap<PlayerId, ConnectedPlayer<dyn Any + Send + Sync>>,
    cx: SystemContext<'a>,
) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + 'a>>
where
    P: RemotePlayer,
    R: for<'b> ReplicaSet<'b>,
{
    let world = cx.world;
    let res = cx.res;
    let spawner = cx.spawner;
    let clock = cx.clock;
    let scope: &'a Scope<'static> = &*cx.scope;

    Box::pin(async move {
        let current_step = session.current_step();

        loop {
            let mut events = session.events::<P::Info, (NetId, P::Input)>(scope)?;
            let first = events.next();

            if first.is_none() {
                break;
            }

            for (cid, event) in first.into_iter().chain(events) {
                match event {
                    Event::ClientConnect(event) => {
                        tracing::info!("New client connection {:?}", cid);
                        let _ = event.accept(scope).await;
                        tracing::debug!("Accepted");
                    }
                    Event::AddPlayer(event) => {
                        tracing::info!("New player @ {:?}", cid);
                        let scope = &*scope;
                        let res = &mut *res;
                        let world = &mut *world;
                        let _ = event
                            .try_accept_with::<PlayerId, _, _, eyre::Report>(
                                |info| {
                                    let pid = id_gen.gen_pid();

                                    tracing::trace!("Generated {:?} for {:?} ", pid, cid);
                                    let player = P::accept(info, pid, world, res, spawner)?;

                                    tracing::info!("{:?}@{:?} spawned", pid, cid);

                                    players.insert(
                                        pid,
                                        ConnectedPlayer {
                                            cid,
                                            player: Box::new(player),
                                        },
                                    );

                                    Ok(pid)
                                },
                                scope,
                            )
                            .await;
                    }
                    Event::Inputs(event) => {
                        tracing::debug!(
                            "Received inputs from {:?} ({}) | {}",
                            cid,
                            event.step(),
                            current_step
                        );
                        for (pid, (nid_res, input)) in event.inputs() {
                            if let Some(player) = players.get_mut(&pid) {
                                let player = player.player.downcast_mut::<P>().unwrap();

                                match nid_res {
                                    Err(err) => {
                                        tracing::error!("{:?}", err)
                                    }
                                    Ok(nid) => {
                                        tracing::trace!("Received inputs from {:?}@{:?}", pid, cid);
                                        if let Some(entity) = mapper.get(nid) {
                                            match world
                                                .query_one_mut::<&mut CommandQueue<P::Command>>(
                                                    entity,
                                                ) {
                                                Err(QueryOneError::NoSuchEntity) => {}
                                                Err(QueryOneError::Unsatisfied) => {
                                                    let mut queue = CommandQueue::new();
                                                    player
                                                        .replicate_input(input, &mut queue, scope);
                                                    world.insert_one(entity, queue);
                                                }
                                                Ok(queue) => {
                                                    player.replicate_input(input, queue, scope);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Event::Disconnected => {
                        tracing::info!("{:?} disconnected", cid);
                        players.retain(|_, player| player.cid != cid);
                    }
                }
            }
        }

        let slice: &[R::ReplicaPack] = R::replicate(id_gen, mapper, world, scope);

        session
            .advance::<Seq<R::Replica>, _>(clock.delta.as_nanos(), slice.iter().copied(), scope)
            .await;

        Ok(())
    })
}

impl ServerSystem {
    #[instrument(skip(self, cx))]
    pub async fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        (self.run_impl)(
            &mut self.session,
            &mut self.id_gen,
            &mut self.mapper,
            &mut self.players,
            cx,
        )
        .await
    }
}
