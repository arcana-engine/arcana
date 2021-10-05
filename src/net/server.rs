use std::{any::Any, future::Future, num::NonZeroU64, pin::Pin};

use alkahest::{Bytes, Pack, Schema, Seq, Unpacked};
use arcana_time::TimeSpan;
use astral::{
    channel::tcp::TcpChannel,
    client_server::{ClientId, Event, PlayerId, ServerSession},
};
use hashbrown::HashMap;
use hecs::{Component, Fetch, Query, World};
use scoped_arena::Scope;
use tokio::net::TcpListener;

use crate::{Res, SystemContext};

use super::{NetId, ReplicaSerde};

pub trait ReplicaSetElem<'a> {
    type Query: Query;
    type Replica: Schema;
    type ReplicaPack: Pack<Self::Replica> + Copy;

    fn replicate(
        item: <<Self::Query as Query>::Fetch as Fetch<'a>>::Item,
        scope: &'a Scope<'_>,
    ) -> Self::ReplicaPack;
}

impl<'a, T> ReplicaSetElem<'a> for ReplicaSerde<T>
where
    T: serde::Serialize + Component,
{
    type Query = &'a T;
    type Replica = Bytes;
    type ReplicaPack = &'a [u8];

    fn replicate(item: &'a T, scope: &'a Scope<'_>) -> &'a [u8] {
        let mut out = Vec::new_in(scope);
        bincode::serialize_into(&mut out, item).expect("Failed to serialize item");
        out.leak()
    }
}

pub trait ReplicaSet<'a> {
    type Replica: Schema;
    type ReplicaPack: Pack<Self::Replica> + Copy;

    fn replicate(world: &'a mut World, scope: &'a Scope<'a>) -> &'a [Self::ReplicaPack];
}

impl<'a> ReplicaSet<'a> for () {
    type Replica = NetId;
    type ReplicaPack = NetId;

    fn replicate(world: &'a mut World, scope: &'a Scope<'_>) -> &'a [NetId] {
        let mut vec = Vec::with_capacity_in(65536, scope);
        vec.extend(world.query_mut::<&NetId>().into_iter().map(|(_, &nid)| nid));
        vec.leak()
    }
}

macro_rules! replica_set_tuple {
    ($($a:ident),+) => {
        impl<'a, $($a),+> ReplicaSet<'a> for ($($a,)+)
        where
            $($a: ReplicaSetElem<'a>,)+
        {
            type Replica = (NetId, $($a::Replica),+);
            type ReplicaPack = (NetId, $($a::ReplicaPack),+);

            fn replicate(world: &'a mut World, scope: &'a Scope<'_>) -> &'a[(NetId, $($a::ReplicaPack),+)] {
                #![allow(non_snake_case)]

                let mut vec = Vec::with_capacity_in(65536, scope);
                vec.extend(
                    world
                        .query_mut::<(&NetId, $($a::Query),+)>()
                        .into_iter()
                        .map(|(_, (&nid, $($a,)+))| (nid, $($a::replicate($a, scope),)+))
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
    type Info: Schema;
    type Input: Schema;

    /// Verifies player info.
    /// On success spawns required entities.
    /// Returns `Self` connected to spawned entities.
    /// On error returns a reason.
    fn accept(
        info: Unpacked<'_, Self::Info>,
        res: &mut Res,
        world: &mut World,
    ) -> eyre::Result<Self>
    where
        Self: Sized;

    /// Apply input from remove player.
    /// Ideally this generates same type of commands as local player controller does.
    fn apply_input(&mut self, input: Unpacked<'_, Self::Input>, res: &mut Res, world: &mut World);
}

pub struct ServerSystem {
    session: ServerSession<TcpChannel, TcpListener>,
    players: HashMap<PlayerId, ConnectedPlayer<dyn Any + Send + Sync>>,
    next_player_id: u64,

    run_impl: for<'r> fn(
        &'r mut ServerSession<TcpChannel, TcpListener>,
        &'r mut u64,
        &'r mut HashMap<PlayerId, ConnectedPlayer<dyn Any + Send + Sync>>,
        SystemContext<'r>,
    ) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + 'r>>,
}

struct ConnectedPlayer<P: ?Sized> {
    cid: ClientId,
    player: Box<P>,
}

impl ServerSystem {
    pub fn new<P, R>(listner: TcpListener, step_delta: TimeSpan) -> Self
    where
        P: RemotePlayer,
        R: for<'a> ReplicaSet<'a>,
    {
        ServerSystem {
            session: ServerSession::new(listner, step_delta.as_nanos()),
            players: HashMap::new(),
            next_player_id: 1,
            run_impl: run_impl::<P, R>,
        }
    }
}

fn run_impl<'a, P, R>(
    session: &'a mut ServerSession<TcpChannel, TcpListener>,
    next_player_id: &'a mut u64,
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
        let events = session.events::<P::Info, P::Input>(scope)?;

        for (cid, event) in events {
            match event {
                Event::ClientConnect(event) => {
                    let _ = event.accept(scope).await;
                }
                Event::AddPlayer(event) => {
                    let scope = &*scope;
                    let res = &mut *res;
                    let world = &mut *world;
                    let _ = event
                        .try_accept_with::<PlayerId, _, _, eyre::Report>(
                            |info| {
                                let player_id = PlayerId(
                                    NonZeroU64::new(*next_player_id).expect("u64 ids exhausted"),
                                );
                                *next_player_id += 1;

                                let player = P::accept(info, res, world)?;
                                players.insert(
                                    player_id,
                                    ConnectedPlayer {
                                        cid,
                                        player: Box::new(player),
                                    },
                                );

                                Ok(player_id)
                            },
                            scope,
                        )
                        .await;
                }
                Event::Inputs(event) => {
                    for (pid, input) in event.inputs() {
                        if let Some(player) = players.get_mut(&pid) {
                            player
                                .player
                                .downcast_mut::<P>()
                                .unwrap()
                                .apply_input(input, res, world);
                        }
                    }
                }
                Event::Disconnected => {
                    players.retain(|_, player| player.cid != cid);
                }
            }
        }

        let slice = R::replicate(world, scope);

        session
            .advance::<Seq<R::Replica>, _>(clock.delta.as_nanos(), slice.iter().copied(), scope)
            .await;

        Ok(())
    })
}

impl ServerSystem {
    pub async fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        (self.run_impl)(
            &mut self.session,
            &mut self.next_player_id,
            &mut self.players,
            cx,
        )
        .await
    }
}
