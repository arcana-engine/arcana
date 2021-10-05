use std::{collections::HashMap, future::Future, pin::Pin};

use alkahest::{Bytes, Pack, Schema, Seq, Unpacked};
use astral::{
    channel::tcp::TcpChannel,
    client_server::{ClientSession, PlayerId},
};
use eyre::Context;
use hecs::{Component, Entity, QueryOneError, World};
use scoped_arena::Scope;
use tokio::net::TcpStream;

use crate::{control::CommandQueue, resources::Res, system::SystemContext, task::Spawner};

use super::{NetId, ReplicaSerde};

pub trait ReplicaSetElem {
    type Component: Component;
    type Replica: Schema;

    fn make(unpacked: Unpacked<'_, Self::Replica>) -> Self::Component;

    #[inline(always)]
    fn replicate(unpacked: Unpacked<'_, Self::Replica>, component: &mut Self::Component) {
        *component = Self::make(unpacked)
    }

    #[inline(always)]
    fn spawn(
        component: &mut Self::Component,
        entity: Entity,
        res: &mut Res,
        spawner: &mut Spawner,
    ) {
        let _ = (component, entity, res, spawner);
    }
}

impl<T> ReplicaSetElem for ReplicaSerde<T>
where
    T: serde::de::DeserializeOwned + Component,
{
    type Component = T;
    type Replica = Bytes;

    fn make(unpacked: &[u8]) -> T {
        bincode::deserialize_from(unpacked).expect("Failed to deserialize item")
    }
}

pub struct EntityMapper {
    entity_by_id: HashMap<NetId, Entity>,
}

pub trait ReplicaSet {
    type Replica: Schema;

    fn replicate(
        unpacked: Unpacked<'_, Seq<Self::Replica>>,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
        mapper: &mut EntityMapper,
    );
}

impl ReplicaSet for () {
    type Replica = NetId;

    fn replicate(
        unpacked: Unpacked<'_, Seq<NetId>>,
        world: &mut World,
        _res: &mut Res,
        _spawner: &mut Spawner,
        mapper: &mut EntityMapper,
    ) {
        for nid in unpacked {
            match mapper.entity_by_id.get(&nid) {
                None => {
                    let entity = world.spawn((nid,));
                    mapper.entity_by_id.insert(nid, entity);
                }
                Some(&entity) => match world.query_one_mut::<&NetId>(entity) {
                    Ok(id) => {
                        debug_assert_eq!(*id, nid);
                    }
                    Err(QueryOneError::Unsatisfied) => {
                        panic!("NetId component was removed on networked entity");
                    }
                    Err(QueryOneError::NoSuchEntity) => {
                        let entity = world.spawn((nid,));
                        mapper.entity_by_id.insert(nid, entity);
                    }
                },
            }
        }
    }
}

macro_rules! replica_set_tuple {
    ($($a:ident),+; $($b:ident),+) => {
        impl<$($a),+> ReplicaSet for ($($a,)+)
        where
            $($a: ReplicaSetElem,)+
        {
            type Replica = (NetId, $($a::Replica),+);

            fn replicate(
                unpacked: Unpacked<'_, Seq<(NetId, $($a::Replica),+)>>,
                world: &mut World,
                res: &mut Res,
                spawner: &mut Spawner,
                mapper: &mut EntityMapper,
            ) {
                #![allow(non_snake_case)]

                for (nid, $($a),+) in unpacked {
                    match mapper.entity_by_id.get(&nid) {
                        None => {
                            let entity = world.spawn((nid, $($a::make($a),)+));
                            mapper.entity_by_id.insert(nid, entity);

                            let ($($a,)+) = world.query_one_mut::<($(&mut $a::Component,)+)>(entity).unwrap();

                            $(
                                $a::spawn($a, entity, res, spawner);
                            )+
                        }
                        Some(&entity) => match world.query_one_mut::<(&NetId, $(&mut $a::Component,)+)>(entity) {
                            Ok((id, $($b, )+)) => {
                                debug_assert_eq!(*id, nid);

                                $(
                                    $a::replicate($a, $b);
                                )+
                            }
                            Err(QueryOneError::Unsatisfied) => {
                                panic!("NetId component was removed on networked entity");
                            }
                            Err(QueryOneError::NoSuchEntity) => {
                                let entity = world.spawn((nid, $($a::make($a),)+));
                                mapper.entity_by_id.insert(nid, entity);

                                let ($($a,)+) = world.query_one_mut::<($(&mut $a::Component,)+)>(entity).unwrap();

                                $(
                                    $a::spawn($a, entity, res, spawner);
                                )+
                            }
                        },
                    }
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
    type Replica: Schema;
    type ReplicaPack: Pack<Self::Replica>;

    fn replicate(queue: &'a CommandQueue<Self>, scope: &'a Scope<'_>) -> Self::ReplicaPack
    where
        Self: Sized;
}

pub struct ClientSystem {
    session: ClientSession<TcpChannel>,
    mapper: EntityMapper,
    controlled: Vec<(Entity, PlayerId)>,
    send_inputs: for<'r> fn(
        &[(Entity, PlayerId)],
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
    pub async fn new<I, R>(stream: TcpStream, scope: &Scope<'_>) -> eyre::Result<Self>
    where
        I: for<'a> InputsReplicate<'a>,
        R: ReplicaSet,
    {
        Ok(ClientSystem {
            session: ClientSession::new(TcpChannel::new(stream), scope).await?,
            mapper: EntityMapper {
                entity_by_id: HashMap::new(),
            },
            controlled: Vec::new(),
            send_inputs: send_inputs::<I>,
            replicate: replicate::<R>,
        })
    }

    pub async fn add_player<P, K>(&mut self, player: K, entity: Entity, scope: &Scope<'_>)
    where
        P: Schema,
        K: Pack<P>,
    {
        let id = self
            .session
            .add_player::<P, PlayerId, _>(player, scope)
            .await
            .ok()
            .flatten()
            .expect("Failed to add player");

        self.controlled.push((entity, id));
    }
}

fn send_inputs<'a, I>(
    controlled: &[(Entity, PlayerId)],
    session: &'a mut ClientSession<TcpChannel>,
    cx: SystemContext<'a>,
) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + 'a>>
where
    I: for<'b> InputsReplicate<'b>,
{
    let scope: &'a Scope<'static> = &*cx.scope;

    let mut inputs = Vec::with_capacity_in(controlled.len(), scope);

    for &(e, id) in controlled {
        let result = unsafe {
            // # Safety
            // `World` is mutably borrowed, no mutable component borrows overlap with these.
            cx.world.get_unchecked::<CommandQueue<I>>(e)
        };
        match result {
            Ok(input) => {
                let input = I::replicate(input, scope);
                inputs.push((id, input));
            }
            Err(err) => {
                tracing::error!(
                    "Failed to fetch command queue from {:?}({:?}) with {}",
                    id,
                    e,
                    err
                );
            }
        }
    }

    Box::pin(async move {
        session
            .send_inputs::<I::Replica, _, _>(inputs, scope)
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
    if let Some(updates) =
        session.advance::<Seq<R::Replica>>(cx.clock.delta.as_nanos(), cx.scope)?
    {
        cx.res.with(|| ServerStep { value: 0 }).value = updates.server_step;

        R::replicate(updates.updates, cx.world, cx.res, cx.spawner, mapper);
    }

    Ok(())
}

impl ClientSystem {
    pub async fn run(&mut self, mut cx: SystemContext<'_>) -> eyre::Result<()> {
        (self.send_inputs)(&self.controlled, &mut self.session, cx.reborrow()).await?;
        (self.replicate)(&mut self.session, &mut self.mapper, cx)
    }
}
