use std::{
    any::{type_name, Any, TypeId},
    collections::{HashMap, VecDeque},
    convert::TryFrom,
    future::Future,
    io::Cursor,
    marker::PhantomData,
    num::NonZeroU64,
    pin::Pin,
};

use alkahest::{Bytes, FixedUsize, Pack, Schema, Unpacked};

use astral::{
    channel::tcp::TcpChannel,
    client_server::{ClientId, Event, PlayerId, ServerSession},
};
use bincode::Options as _;
use bitsetium::{BitEmpty, BitSet, BitTestNone};
use hecs::{Component, Entity, Fetch, Query, QueryOneError, World};
use scoped_arena::Scope;
use tokio::net::TcpListener;
use tracing::instrument;

use crate::{CommandQueue, Res, Spawner, SystemContext};

use super::{EntityHeader, EntityMapper, IdGen, InputSchema, NetId, WorldPacked, WorldSchema};

/// Component to signal that entity is owned by server and must be replicated to connected clients.
pub struct ServerOwned;

pub type DescriptorFetchItem<'a, T> =
    <<<T as Descriptor>::Query as Query>::Fetch as Fetch<'a>>::Item;

pub type DescriptorPackType<'a, T> = <T as DescriptorPack<'a>>::Pack;

pub enum Replicate<T> {
    Unmodified,
    Modified(T),
}

pub trait DescriptorPack<'a> {
    /// Data ready to be packed.
    type Pack: serde::Serialize + 'a;
}

pub trait Descriptor: for<'a> DescriptorPack<'a> + 'static {
    /// Query to perform for replication.
    type Query: Query;

    /// Component that contains history of the data processed by this descriptor.
    /// Can be used to diff-compression.
    type History: Component;

    /// Make history point.
    fn history(item: DescriptorFetchItem<'_, Self>) -> Self::History;

    /// Replicate value.
    fn replicate<'a>(
        item: DescriptorFetchItem<'a, Self>,
        history: Option<&Self::History>,
        scope: &'a Scope<'_>,
    ) -> Replicate<DescriptorPackType<'a, Self>>;
}

pub trait SelfDescriptor: for<'a> DescriptorPack<'a> + Component {
    /// Get descriptor
    fn descriptor() -> PhantomData<Self> {
        PhantomData
    }

    /// Component that contains history of the data processed by this descriptor.
    /// Can be used to diff-compression.
    type History: Component;

    /// Make history point.
    fn history(&self) -> Self::History;

    /// Replicate value.
    fn replicate<'a>(
        &'a self,
        history: Option<&Self::History>,
        scope: &'a Scope<'_>,
    ) -> Replicate<DescriptorPackType<'a, Self>>;
}

impl<'a, T> DescriptorPack<'a> for PhantomData<T>
where
    T: SelfDescriptor,
{
    type Pack = DescriptorPackType<'a, T>;
}

impl<T> Descriptor for PhantomData<T>
where
    T: SelfDescriptor,
{
    type Query = &'static T;

    type History = T::History;

    fn history(item: &T) -> Self::History {
        T::history(item)
    }

    fn replicate<'a>(
        item: &'a T,
        history: Option<&Self::History>,
        scope: &'a Scope<'_>,
    ) -> Replicate<DescriptorPackType<'a, Self>> {
        T::replicate(item, history, scope)
    }
}

pub trait TrivialDescriptor: Component + serde::Serialize + Clone + PartialEq {}

impl<'a, T> DescriptorPack<'a> for T
where
    T: TrivialDescriptor,
{
    type Pack = &'a Self;
}

impl<T> SelfDescriptor for T
where
    T: TrivialDescriptor,
{
    type History = Self;

    fn history(&self) -> Self {
        self.clone()
    }

    fn replicate<'a>(
        &'a self,
        history: Option<&Self>,
        _scope: &'a Scope<'_>,
    ) -> Replicate<&'a Self> {
        match history {
            Some(history) if *history == *self => Replicate::Unmodified,
            _ => Replicate::Modified(self),
        }
    }
}

struct PlayerIdDescriptor {}

impl DescriptorPack<'_> for PlayerIdDescriptor {
    type Pack = NonZeroU64;
}

impl Descriptor for PlayerIdDescriptor {
    type Query = &'static PlayerId;
    type History = PlayerId;

    fn history(item: &PlayerId) -> Self::History {
        *item
    }

    fn replicate<'a>(
        item: &'a PlayerId,
        history: Option<&PlayerId>,
        _scope: &'a Scope<'_>,
    ) -> Replicate<NonZeroU64> {
        match history {
            Some(history) if *history == *item => Replicate::Unmodified,
            _ => Replicate::Modified(item.0),
        }
    }
}

#[repr(transparent)]
struct History<T> {
    buf: VecDeque<Option<T>>,
}

/// Missing history entry.
struct Missing;

/// Result of history tracking.
#[derive(Debug)]
enum TrackReplicate<T> {
    Unmodified,
    Modified(T),
    Removed,
}

impl<T> TrackReplicate<T> {
    pub fn write_mask<B>(&self, idx: usize, mask: &mut B)
    where
        B: BitSet,
    {
        match self {
            TrackReplicate::Unmodified => {}
            TrackReplicate::Modified(_) => {
                mask.set(idx * 2);
                mask.set(idx * 2 + 1);
            }
            TrackReplicate::Removed => {
                mask.set(idx * 2);
            }
        }
    }
}

impl<T> History<T> {
    fn new() -> Self {
        History {
            buf: VecDeque::with_capacity(8),
        }
    }

    fn track_replicate<'a, D>(
        &self,
        back: u64,
        item: Option<DescriptorFetchItem<'a, D>>,
        scope: &'a Scope<'_>,
    ) -> TrackReplicate<DescriptorPackType<'a, D>>
    where
        D: Descriptor<History = T>,
    {
        let old = self.fetch(back);

        let replicate = match (item, old) {
            (None, Ok(None)) => return TrackReplicate::Unmodified,
            (None, Ok(Some(_)) | Err(_)) => {
                return TrackReplicate::Removed;
            }
            (Some(item), Err(_)) => D::replicate(item, None, scope),
            (Some(item), Ok(old)) => D::replicate(item, old, scope),
        };

        match replicate {
            Replicate::Unmodified => TrackReplicate::Unmodified,
            Replicate::Modified(pack) => TrackReplicate::Modified(pack),
        }
    }

    fn fetch(&self, back: u64) -> Result<Option<&T>, Missing> {
        match usize::try_from(back) {
            Err(_) => Err(Missing),
            Ok(back) => {
                if self.buf.len() <= back {
                    Err(Missing)
                } else {
                    Ok(self.buf[back].as_ref())
                }
            }
        }
    }

    fn add(&mut self, value: Option<T>) {
        self.buf.push_front(value)
    }
}

struct ServerEntity<T> {
    nid: NetId,
    history: T,
}

#[derive(Clone, Copy)]
struct WorldReplicaPack<'a, T, B> {
    world: &'a World,
    scope: &'a Scope<'a>,
    back: u64,
    marker: PhantomData<fn(T, B)>,
}

impl<B, R> Pack<WorldSchema> for WorldReplicaPack<'_, R, B>
where
    B: BitEmpty + BitTestNone + BitSet + serde::Serialize,
    R: Replicator,
{
    fn pack(self, offset: usize, output: &mut [u8]) -> (WorldPacked, usize) {
        <R as Replicator>::pack::<B>(self.world, self.scope, self.back, offset, output)
    }
}

trait Replicator {
    fn init_entities(
        world: &mut World,
        id_gen: &mut IdGen,
        mapper: &mut EntityMapper,
        scope: &Scope<'_>,
    );

    fn update_history(world: &mut World);

    fn pack<BITSET>(
        world: &World,
        scope: &Scope<'_>,
        back: u64,
        offset: usize,
        output: &mut [u8],
    ) -> (WorldPacked, usize)
    where
        BITSET: BitEmpty + BitTestNone + BitSet + serde::Serialize;
}

impl Replicator for () {
    fn init_entities(
        world: &mut World,
        id_gen: &mut IdGen,
        mapper: &mut EntityMapper,
        scope: &Scope<'_>,
    ) {
        let query = world
            .query_mut::<()>()
            .with::<ServerOwned>()
            .without::<ServerEntity<(History<PlayerId>,)>>();

        let entities: &[Entity] = scope.to_scope_from_iter(query.into_iter().map(|(e, ())| e));

        for &e in entities {
            let nid = mapper.new_nid(id_gen, e);

            world
                .insert_one(
                    e,
                    ServerEntity {
                        nid,
                        history: (History::<PlayerId>::new(),),
                    },
                )
                .unwrap();
        }
    }

    fn update_history(world: &mut World) {
        let query =
            world.query_mut::<(&mut ServerEntity<(History<PlayerId>,)>, Option<&PlayerId>)>();

        for (_, (server, pid)) in query {
            #[allow(non_snake_case)]
            let (PlayerIdDescriptor,) = &mut server.history;

            PlayerIdDescriptor.add(pid.map(PlayerIdDescriptor::history));
        }
    }

    fn pack<BITSET>(
        world: &World,
        scope: &Scope<'_>,
        back: u64,
        offset: usize,
        output: &mut [u8],
    ) -> (WorldPacked, usize)
    where
        BITSET: BitEmpty + BitTestNone + BitSet + serde::Serialize,
    {
        let opts = bincode::DefaultOptions::new().allow_trailing_bytes();

        let mut query = world.query::<(&ServerEntity<(History<PlayerId>,)>, Option<&PlayerId>)>();

        let mut cursor = std::io::Cursor::new(output);

        for (_, (server, pid)) in query.iter() {
            let mut header = EntityHeader {
                nid: server.nid,
                mask: BITSET::empty(),
            };

            #[allow(non_snake_case)]
            let (PlayerIdDescriptor,) = &server.history;

            // Begin for each component + player_id.

            let pid = PlayerIdDescriptor.track_replicate::<PlayerIdDescriptor>(back, pid, scope);
            pid.write_mask(0, &mut header.mask);

            // End for each component + player_id.

            if !header.mask.test_none() {
                opts.serialize_into(&mut cursor, &header).unwrap();

                // Begin for each component + player_id.

                if let TrackReplicate::Modified(pid) = pid {
                    opts.serialize_into(&mut cursor, &pid).unwrap();
                }

                // End for each component + player_id.
            }
        }

        let len = cursor.position() as usize;

        (
            WorldPacked {
                offset: offset as FixedUsize,
                len: len as FixedUsize,
            },
            len,
        )
    }
}

/// Data associated with spawned player.
/// This may be as simple as controlled entity.
/// Or any kind of data structure.
pub trait RemotePlayer: Send + Sync + 'static {
    type Command: Component + serde::de::DeserializeOwned;
    type Info: serde::de::DeserializeOwned;

    /// Verifies player info.
    /// On success spawns required entities.
    /// Returns `Self` connected to spawned entities.
    /// On error returns a reason.
    fn accept(
        info: Self::Info,
        pid: PlayerId,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
    ) -> eyre::Result<Self>
    where
        Self: Sized;
}

pub struct ServerBuilder<R> {
    ids: Vec<TypeId>,
    marker: PhantomData<fn(R)>,
}

impl ServerBuilder<()> {
    pub fn new() -> Self {
        ServerBuilder {
            ids: Vec::new(),
            marker: PhantomData,
        }
    }

    pub fn with<T>(mut self, descriptor: T) -> ServerBuilder<(T,)>
    where
        T: 'static,
    {
        drop(descriptor);
        let tid = TypeId::of::<T>();
        assert!(
            !self.ids.contains(&tid),
            "Duplicate replica descriptor '{}'",
            type_name::<T>()
        );
        self.ids.push(tid);
        ServerBuilder {
            ids: self.ids,
            marker: PhantomData,
        }
    }

    pub fn build<P>(&self, listener: TcpListener) -> ServerSystem
    where
        P: RemotePlayer,
    {
        ServerSystem::new::<P, ()>(listener)
    }
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
    pub fn builder() -> ServerBuilder<()> {
        ServerBuilder::new()
    }

    fn new<P, R>(listener: TcpListener) -> Self
    where
        P: RemotePlayer,
        R: Replicator,
    {
        ServerSystem {
            session: ServerSession::new(listener),
            players: HashMap::new(),
            id_gen: IdGen::new(),
            mapper: EntityMapper::new(),
            run_impl: run_impl::<P, R, u32>,
        }
    }
}

fn run_impl<'a, P, R, B>(
    session: &'a mut ServerSession<TcpChannel, TcpListener>,
    id_gen: &'a mut IdGen,
    mapper: &'a mut EntityMapper,
    players: &'a mut HashMap<PlayerId, ConnectedPlayer<dyn Any + Send + Sync>>,
    cx: SystemContext<'a>,
) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + 'a>>
where
    P: RemotePlayer,
    R: Replicator,
    B: BitEmpty + BitTestNone + BitSet + serde::Serialize,
{
    let world = cx.world;
    let res = cx.res;
    let spawner = cx.spawner;
    let scope: &'a Scope<'static> = &*cx.scope;
    let opts = bincode::DefaultOptions::new().allow_trailing_bytes();

    let current_step = session.current_step();

    Box::pin(async move {
        loop {
            let mut events = session.events::<Bytes, (NetId, InputSchema)>(scope)?;
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

                                    let info = opts.deserialize(info)?;
                                    let player = P::accept(info, pid, world, res, spawner)?;

                                    tracing::info!("{:?}@{:?} accepted", pid, cid);

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
                            if players.contains_key(&pid) {
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
                                                    let mut queue =
                                                        CommandQueue::<P::Command>::new();
                                                    let mut cursor = Cursor::new(input);

                                                    while cursor.get_ref().len()
                                                        > cursor.position() as usize
                                                    {
                                                        match opts.deserialize_from(&mut cursor) {
                                                            Err(err) => {
                                                                tracing::error!(
                                                                    "Invalid input from {:?}@{:?}: {}",
                                                                    pid,
                                                                    cid,
                                                                    err
                                                                );
                                                            }
                                                            Ok(cmd) => {
                                                                queue.add(cmd);
                                                            }
                                                        }
                                                    }

                                                    world.insert_one(entity, queue).unwrap();
                                                }
                                                Ok(queue) => {
                                                    let mut cursor = Cursor::new(input);

                                                    while cursor.get_ref().len()
                                                        > cursor.position() as usize
                                                    {
                                                        match opts.deserialize_from(&mut cursor) {
                                                            Err(err) => {
                                                                tracing::error!(
                                                                    "Invalid input from {:?}@{:?}: {}",
                                                                    pid,
                                                                    cid,
                                                                    err
                                                                );
                                                            }
                                                            Ok(cmd) => {
                                                                queue.add(cmd);
                                                            }
                                                        }
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

        R::init_entities(world, id_gen, mapper, scope);

        session
            .advance::<WorldSchema, _, _>(
                |back| WorldReplicaPack::<R, B> {
                    world,
                    scope,
                    back,
                    marker: PhantomData,
                },
                scope,
            )
            .await;

        R::update_history(world);

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

macro_rules! for_tuple {
    ($($t:ident $b:ident),+ $(,)?) => {
        impl<$( $t ),+> ServerBuilder<($( $t, )+)> {
            pub fn with<T>(mut self, descriptor: T) -> ServerBuilder<($($t,)+ T,)>
            where
                T: 'static,
            {
                drop(descriptor);
                let tid = TypeId::of::<T>();
                assert!(
                    !self.ids.contains(&tid),
                    "Duplicate replica descriptor {}",
                    type_name::<T>()
                );
                self.ids.push(tid);
                ServerBuilder {
                    ids: self.ids,
                    marker: PhantomData,
                }
            }

            pub fn build<P>(&self, listener: TcpListener) -> ServerSystem
            where
                P: RemotePlayer,
                $(
                    $t: Descriptor,
                )+
            {
                ServerSystem::new::<P, ($($t,)+)>(listener)
            }
        }

        impl<$($t,)+> Replicator for ($($t,)+)
        where
            $(
                $t: Descriptor,
            )+
        {
            fn init_entities(
                world: &mut World,
                id_gen: &mut IdGen,
                mapper: &mut EntityMapper,
                scope: &Scope<'_>,
            ) {
                let query = world
                    .query_mut::<()>()
                    .with::<ServerOwned>()
                    .without::<ServerEntity<( $( History<$t::History>, )+ History<PlayerId>,)>>();

                let entities: &[Entity] = scope.to_scope_from_iter(query.into_iter().map(|(e, ())| e));

                for &e in entities {
                    let nid = mapper.new_nid(id_gen, e);

                    world.insert_one(
                        e,
                        ServerEntity {
                            nid,
                            history: ($( History::<$t::History>::new(), )+ History::<PlayerId>::new(),),
                        },
                    ).unwrap();
                }
            }

            fn update_history(world: &mut World) {
                let query =
                    world.query_mut::<(&mut ServerEntity<( $( History<$t::History>, )+ History<PlayerId>,)>, $( Option<$t::Query>, )+ Option<&PlayerId>)>();

                for (_, (server, $( $b, )+ pid)) in query {
                    #[allow(non_snake_case)]
                    let ($( $t, )+ PlayerIdDescriptor,) = &mut server.history;

                    $(
                        $t.add($b.map($t::history));
                    )+

                    PlayerIdDescriptor.add(pid.map(PlayerIdDescriptor::history));
                }
            }

            fn pack<BITSET>(
                world: &World,
                scope: &Scope<'_>,
                back: u64,
                offset: usize, output: &mut [u8]) -> (WorldPacked, usize)
            where
                BITSET: BitEmpty + BitTestNone + BitSet + serde::Serialize,
            $(
                $t: Descriptor,
            )+
            {
                let opts = bincode::DefaultOptions::new().allow_trailing_bytes();

                let mut query = world.query::<(&ServerEntity<($( History<$t::History>, )+ History<PlayerId>,)>, $( Option<$t::Query>, )+ Option<&PlayerId>)>();

                let mut cursor = std::io::Cursor::new(output);

                for (_, (server, $( $b, )+ pid)) in query.iter() {
                    let mut header = EntityHeader {
                        nid: server.nid,
                        mask: BITSET::empty(),
                    };
                    let mut mask_idx = 0;

                    #[allow(non_snake_case)]
                    let ($( $t, )+ PlayerIdDescriptor,) = &server.history;

                    // Begin for each component + player_id.

                    $(
                        let $b: TrackReplicate<_> = $t.track_replicate::<$t>(back, $b, scope);
                        $b.write_mask(mask_idx, &mut header.mask);
                        mask_idx += 1;
                    )+

                    let pid: TrackReplicate<_> = PlayerIdDescriptor.track_replicate::<PlayerIdDescriptor>(back, pid, scope);
                    pid.write_mask(mask_idx, &mut header.mask);

                    // End for each component + player_id.

                    if !header.mask.test_none() {
                        opts.serialize_into(&mut cursor, &header).unwrap();

                        // Begin for each component + player_id.

                        $(
                            if let TrackReplicate::Modified($b) = $b {
                                opts.serialize_into(&mut cursor, &$b).unwrap();
                            }
                        )+

                        if let TrackReplicate::Modified(pid) = pid {
                            opts.serialize_into(&mut cursor, &pid).unwrap();
                        }

                        // End for each component + player_id.
                    }
                }

                let len = cursor.position() as usize;

                // tracing::error!("DATA: {:?}", &cursor.into_inner()[..len]);

                (
                    WorldPacked {
                        offset: offset as u32,
                        len: len as u32,
                    },
                    len,
                )
            }
        }
    };
}

for_tuple!(A a);
for_tuple!(A a, B b);
for_tuple!(A a, B b, C c);
for_tuple!(A a, B b, C c, D d);
for_tuple!(A a, B b, C c, D d, E e);
for_tuple!(A a, B b, C c, D d, E e, F f);
for_tuple!(A a, B b, C c, D d, E e, F f, G g);
for_tuple!(A a, B b, C c, D d, E e, F f, G g, H h);
for_tuple!(A a, B b, C c, D d, E e, F f, G g, H h, I i);
