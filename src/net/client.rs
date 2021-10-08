use std::{future::Future, pin::Pin};

use alkahest::{Pack, Schema, Seq, SeqUnpacked, Unpacked};
use astral::{
    channel::tcp::TcpChannel,
    client_server::{ClientSession, MaybePlayerId, PlayerId},
};
use eyre::Context;
use hashbrown::HashSet;
use hecs::{Component, Entity, QueryOneError, World};
use scoped_arena::Scope;
use tokio::net::{TcpStream, ToSocketAddrs};
use tracing::instrument;

use crate::{control::CommandQueue, resources::Res, system::SystemContext, task::Spawner};

use super::{EntityMapper, NetId};

pub trait ReplicaSetElem {
    type Component: Component;
    type Replica: Schema;

    fn build(unpacked: Unpacked<'_, Self::Replica>) -> Self::Component;

    #[inline(always)]
    fn replicate(unpacked: Unpacked<'_, Self::Replica>, component: &mut Self::Component) {
        *component = Self::build(unpacked)
    }

    #[inline(always)]
    fn pre_insert(
        component: &mut Self::Component,
        entity: Entity,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
    ) {
        (entity, world, res, spawner);
    }
}

pub trait ReplicaSet {
    type Replica: Schema;

    fn replicate(
        unpacked: Unpacked<'_, Seq<Self::Replica>>,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
        mapper: &mut EntityMapper,
        scope: &Scope<'_>,
    );
}

impl ReplicaSet for () {
    type Replica = (NetId, MaybePlayerId);

    fn replicate(
        unpacked: SeqUnpacked<'_, (NetId, MaybePlayerId)>,
        world: &mut World,
        _res: &mut Res,
        _spawner: &mut Spawner,
        mapper: &mut EntityMapper,
        scope: &Scope<'_>,
    ) {
        let mut set = HashSet::new_in(scope);

        tracing::debug!("Replicate {} entities", unpacked.len());

        for (nid_res, pid_opt) in unpacked {
            let nid = match nid_res {
                Err(err) => {
                    tracing::error!("{:?}", err);
                    continue;
                }
                Ok(nid) => nid,
            };

            set.insert(nid);

            let entity = mapper.get_or_spawn(world, nid);

            if let Some(pid) = pid_opt {
                world.insert_one(entity, pid);
            }
        }

        let mut despawn = Vec::new_in(scope);
        for (e, nid) in world.query_mut::<&NetId>() {
            if !set.contains(nid) {
                despawn.push(e);
            }
        }

        for e in despawn {
            world.despawn(e).unwrap();
        }
    }
}

macro_rules! replica_set_tuple {
    ($($a:ident),+; $($b:ident),+) => {
        impl<$($a),+> ReplicaSet for ($($a,)+)
        where
            $($a: ReplicaSetElem,)+
        {
            type Replica = (NetId, MaybePlayerId, $(Option<$a::Replica>),+);

            fn replicate(
                unpacked: SeqUnpacked<'_, (NetId, MaybePlayerId, $(Option<$a::Replica>),+)>,
                world: &mut World,
                res: &mut Res,
                spawner: &mut Spawner,
                mapper: &mut EntityMapper,
                scope: &Scope<'_>,
            ) {
                #![allow(non_snake_case)]

                tracing::debug!("Replicate {} entities", unpacked.len());

                let mut set = HashSet::new_in(scope);

                for (nid_res, pid_opt, $($a),+) in unpacked {
                    let nid = match nid_res {
                        Err(err) => {
                            tracing::error!("{:?}", err);
                            continue;
                        }
                        Ok(nid) => nid,
                    };

                    set.insert(nid);

                    let entity = mapper.get_or_spawn(world, nid);

                    match world.query_one_mut::<(&NetId, Option<&mut PlayerId>, $(Option<&mut $a::Component>,)+)>(entity) {
                        Ok((nid_, pid_opt_, $($b, )+)) => {
                            debug_assert_eq!(*nid_, nid);

                            enum Action<I, R> {
                                Insert(I),
                                Remove(R),
                                Nothing,
                            }

                            let pid = match (pid_opt, pid_opt_) {
                                (None, None) => Action::Nothing,
                                (None, Some(_)) => Action::Remove(move |world: &mut World| {
                                    world.remove_one::<PlayerId>(entity);
                                }),
                                (Some(pid), None) => Action::Insert(move |world: &mut World| {
                                    world.insert_one(entity, pid).unwrap();
                                }),
                                (Some(pid), Some(pid_)) => { *pid_ = pid; Action::Nothing }
                            };

                            let ($($b,)+) = ($(
                                match ($a, $b) {
                                    (None, None) => Action::Nothing,
                                    (None, Some($b)) => Action::Remove(move |world: &mut World| {
                                        world.remove_one::<$a::Component>(entity);
                                    }),
                                    (Some($a), None) => Action::Insert(move |world: &mut World, res: &mut Res, spawner: &mut Spawner| {
                                        let mut $b = $a::build($a);
                                        $a::pre_insert(&mut $b, entity, world, res, spawner);
                                        world.insert_one(entity, $b).unwrap();
                                    }),
                                    (Some($a), Some($b)) => {$a::replicate($a, $b); Action::Nothing }
                                },
                            )+);

                            match pid {
                                Action::Insert(f) => f(world),
                                Action::Remove(f) => f(world),
                                Action::Nothing => {}
                            }

                            $(
                                match $b {
                                    Action::Insert(f) => f(world, res, spawner),
                                    Action::Remove(f) => f(world),
                                    Action::Nothing => {}
                                }
                            )+
                        }
                        Err(QueryOneError::Unsatisfied) => unreachable!(),
                        Err(QueryOneError::NoSuchEntity) => unreachable!(),
                    }
                }

                let mut despawn = Vec::new_in(scope);
                for (e, nid) in world.query_mut::<&NetId>() {
                    if !set.contains(nid) {
                        despawn.push(e);
                    }
                }

                for e in despawn {
                    world.despawn(e).unwrap();
                }
            }
        }
    };
}

replica_set_tuple!(A1; a2);
replica_set_tuple!(A1, B1; a2, b2);
replica_set_tuple!(A1, B1, C1; a2, b2, c2);
replica_set_tuple!(A1, B1, C1, D1; a2, b2, c2, d2);

pub struct ServerStep {
    pub value: u64,
}

pub trait InputsReplicate<'a>: Send + Sync + 'static {
    type Command: Component;
    type Replica: Schema;
    type ReplicaPack: Pack<Self::Replica>;

    fn replicate(
        queue: &mut CommandQueue<Self::Command>,
        scope: &'a Scope<'_>,
    ) -> Self::ReplicaPack;
}

pub struct ClientSystem {
    session: Option<ClientSession<TcpChannel>>,
    mapper: EntityMapper,
    controlled: HashSet<PlayerId>,
    send_inputs: for<'r> fn(
        &HashSet<PlayerId>,
        &'r mut ClientSession<TcpChannel>,
        SystemContext<'r>,
    ) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + 'r>>,
    replicate: fn(
        &mut ClientSession<TcpChannel>,
        &mut EntityMapper,
        SystemContext<'_>,
    ) -> eyre::Result<()>,
}

impl ClientSystem {
    pub fn new<I, R>() -> Self
    where
        I: for<'a> InputsReplicate<'a>,
        R: ReplicaSet,
    {
        ClientSystem {
            session: None,
            mapper: EntityMapper::new(),
            controlled: HashSet::new(),
            send_inputs: send_inputs::<I>,
            replicate: replicate::<R>,
        }
    }

    pub async fn connect(
        &mut self,
        addr: impl ToSocketAddrs,
        scope: &Scope<'_>,
    ) -> eyre::Result<()> {
        let stream = TcpStream::connect(addr).await?;
        self.connect_with_stream(stream, scope).await
    }

    pub async fn connect_with_stream(
        &mut self,
        stream: TcpStream,
        scope: &Scope<'_>,
    ) -> eyre::Result<()> {
        let session = ClientSession::new(TcpChannel::new(stream), scope).await?;

        if let Some(session) = &mut self.session {
            // TODO: Leave session.
        }

        self.session = Some(session);

        Ok(())
    }

    pub async fn add_player<P, K>(&mut self, player: K, scope: &Scope<'_>) -> eyre::Result<PlayerId>
    where
        P: Schema,
        K: Pack<P> + 'static,
    {
        let id = self
            .session
            .as_mut()
            .expect("Attempt to add player in disconnected ClientSystem")
            .add_player::<P, PlayerId, _>(player, scope)
            .await
            .map_or_else(
                |err| Err(eyre::Report::from(err)),
                |res| res.map_err(eyre::Report::from),
            )
            .wrap_err_with(|| eyre::eyre!("Failed to add player"))?;

        let no_collision = self.controlled.insert(id);
        if !no_collision {
            return Err(eyre::eyre!("PlayerId({:?}) collision detected", id));
        }
        Ok(id)
    }
}

fn send_inputs<'a, I>(
    controlled: &HashSet<PlayerId>,
    session: &'a mut ClientSession<TcpChannel>,
    cx: SystemContext<'a>,
) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + 'a>>
where
    I: for<'b> InputsReplicate<'b>,
{
    let scope: &'a Scope<'static> = &*cx.scope;

    let mut inputs = Vec::with_capacity_in(controlled.len(), scope);

    for (_, (pid, nid, queue)) in cx
        .world
        .query_mut::<(&PlayerId, &NetId, &mut CommandQueue<I::Command>)>()
    {
        if controlled.contains(pid) {
            let input = I::replicate(queue, scope);
            inputs.push((*pid, (*nid, input)));
        }
    }

    tracing::debug!("Sending input ({})", session.current_step());
    Box::pin(async move {
        session
            .send_inputs::<(NetId, I::Replica), _, _>(inputs, scope)
            .await
            .wrap_err("Failed to send inputs from client to server")
    })
}

fn replicate<R>(
    session: &mut ClientSession<TcpChannel>,
    mapper: &mut EntityMapper,
    cx: SystemContext<'_>,
) -> eyre::Result<()>
where
    R: ReplicaSet,
{
    let updates = session.advance::<Seq<R::Replica>>(cx.clock.delta.as_nanos(), cx.scope)?;
    if let Some(updates) = updates {
        tracing::debug!("Received updates ({})", updates.server_step);

        cx.res.with(|| ServerStep { value: 0 }).value = updates.server_step;

        R::replicate(
            updates.updates,
            cx.world,
            cx.res,
            cx.spawner,
            mapper,
            cx.scope,
        );
    }

    Ok(())
}

impl ClientSystem {
    #[instrument(skip(self, cx))]
    pub async fn run(&mut self, mut cx: SystemContext<'_>) -> eyre::Result<()> {
        if let Some(session) = &mut self.session {
            (self.send_inputs)(&self.controlled, session, cx.reborrow()).await?;
            (self.replicate)(session, &mut self.mapper, cx)?;
        }
        Ok(())
    }
}
