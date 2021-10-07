use std::{any::Any, future::Future, num::NonZeroU64, pin::Pin};

use alkahest::{Bytes, Pack, Schema, Seq, Unpacked};
use arcana_time::TimeSpan;
use astral::{
    channel::tcp::TcpChannel,
    client_server::{ClientId, Event, PlayerId, ServerSession},
};
use hashbrown::HashMap;
use hecs::{Component, Entity, Fetch, Query, QueryOneError, World};
use scoped_arena::Scope;
use tokio::net::TcpListener;
use tracing::instrument;

use crate::{CommandQueue, Res, SystemContext};

use super::{EntityMapper, NetId, ReplicaPrefabSerde, ReplicaSerde};

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
    type Query: Query;
    type Replica: Schema;
    type ReplicaPack: Pack<Self::Replica> + Copy + 'a;

    fn replicate<'b>(
        item: <<Self::Query as Query>::Fetch as Fetch<'b>>::Item,
        scope: &'a Scope<'_>,
    ) -> Self::ReplicaPack;
}

impl<'a, T> ReplicaSetElem<'a> for ReplicaSerde<T>
where
    T: serde::Serialize + Component,
{
    type Query = &'static T;
    type Replica = Bytes;
    type ReplicaPack = &'a [u8];

    fn replicate<'b>(item: &'b T, scope: &'a Scope<'_>) -> &'a [u8] {
        let mut out = Vec::new_in(scope);
        bincode::serialize_into(&mut out, item).expect("Failed to serialize item");
        out.leak()
    }
}

impl<'a, T> ReplicaSetElem<'a> for ReplicaPrefabSerde<T>
where
    T: serde::Serialize + Component,
{
    type Query = &'static T;
    type Replica = Bytes;
    type ReplicaPack = &'a [u8];

    fn replicate<'b>(item: &'b T, scope: &'a Scope<'_>) -> &'a [u8] {
        let mut out = Vec::new_in(scope);
        bincode::serialize_into(&mut out, item).expect("Failed to serialize item");
        out.leak()
    }
}

pub trait ReplicaSet<'a> {
    type Replica: Schema;
    type ReplicaPack: Pack<Self::Replica> + Copy;

    fn replicate(
        id_gen: &mut IdGen,
        mapper: &mut EntityMapper,
        world: &mut World,
        scope: &'a Scope<'a>,
    ) -> &'a [Self::ReplicaPack];
}

impl<'a> ReplicaSet<'a> for () {
    type Replica = NetId;
    type ReplicaPack = NetId;

    fn replicate(
        id_gen: &mut IdGen,
        mapper: &mut EntityMapper,
        world: &mut World,
        scope: &'a Scope<'_>,
    ) -> &'a [NetId] {
        let mut new_nids = Vec::new_in(scope);

        let mut vec = Vec::with_capacity_in(65536, scope);
        vec.extend(
            world
                .query_mut::<Option<&NetId>>()
                .with::<ServerOwned>()
                .into_iter()
                .map(|(e, nid_opt)| {
                    let nid = match nid_opt {
                        None => {
                            let nid = id_gen.gen_nid();
                            new_nids.push((e, nid));
                            nid
                        }
                        Some(nid) => *nid,
                    };
                    nid
                }),
        );

        for (e, nid) in new_nids {
            mapper.entity_by_id.insert(nid, e);
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
            type Replica = (NetId, $(Option<$a::Replica>),+);
            type ReplicaPack = (NetId, $(Option<$a::ReplicaPack>),+);

            fn replicate(
                id_gen: &mut IdGen,
                mapper: &mut EntityMapper,
                world: &mut World,
                scope: &'a Scope<'_>
            ) -> &'a[(NetId, $(Option<$a::ReplicaPack>),+)] {
                #![allow(non_snake_case)]

                let mut new_nids = Vec::new_in(scope);

                let mut vec = Vec::with_capacity_in(65536, scope);
                vec.extend(
                    world
                        .query_mut::<(Option<&NetId>, $(Option<$a::Query>),+)>()
                        .with::<ServerOwned>()
                        .into_iter()
                        .map(|(e, (nid_opt, $($a,)+))| {
                            let nid = match nid_opt {
                                None => {
                                    let nid = id_gen.gen_nid();
                                    new_nids.push((e, nid));
                                    nid
                                },
                                Some(nid) => *nid,
                            };

                            (nid, $($a.map(|$a| $a::replicate($a, scope)),)+)
                        })
                );

                for (e, nid) in new_nids {
                    mapper.entity_by_id.insert(nid, e);
                    world.insert_one(e, nid);
                }

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
        res: &mut Res,
        world: &mut World,
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
    let scope: &'a Scope<'static> = &*cx.scope;
    let res = cx.res;
    let world = cx.world;
    let clock = cx.clock;

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
                                    let player = P::accept(info, pid, res, world)?;

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
                        for (pid, (nid_opt, input)) in event.inputs() {
                            if let Some(player) = players.get_mut(&pid) {
                                let player = player.player.downcast_mut::<P>().unwrap();

                                match nid_opt {
                                    None => {
                                        tracing::error!("Zero NetId received with client inputs")
                                    }
                                    Some(nid) => {
                                        tracing::trace!("Received inputs from {:?}@{:?}", pid, cid);
                                        match mapper.entity_by_id.get(&nid) {
                                            None => {}
                                            Some(&entity) => {
                                                match world
                                                    .query_one_mut::<&mut CommandQueue<P::Command>>(
                                                        entity,
                                                    ) {
                                                    Err(QueryOneError::NoSuchEntity) => {}
                                                    Err(QueryOneError::Unsatisfied) => {
                                                        let mut queue = CommandQueue::new();
                                                        player.replicate_input(
                                                            input, &mut queue, scope,
                                                        );
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
                    }
                    Event::Disconnected => {
                        tracing::info!("{:?} disconnected", cid);
                        players.retain(|_, player| player.cid != cid);
                    }
                }
            }
        }

        let slice = R::replicate(id_gen, mapper, world, scope);

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
