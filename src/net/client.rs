use std::{
    any::{type_name, TypeId},
    future::Future,
    io::Cursor,
    marker::PhantomData,
    num::NonZeroU64,
    pin::Pin,
};

use alkahest::{Bytes, FixedUsize, Pack};
use astral::{
    channel::tcp::TcpChannel,
    client_server::{ClientSession, PlayerId},
};
use bincode::Options as _;
use bitsetium::BitTest;
use eyre::Context;
use hashbrown::HashSet;
use hecs::{Component, Entity, Fetch, Query, World};
use scoped_arena::Scope;
use tokio::net::{TcpStream, ToSocketAddrs};
use tracing::instrument;

use crate::{control::CommandQueue, resources::Res, system::SystemContext, task::Spawner};

use super::{
    EntityHeader, EntityMapper, InputPacked, InputSchema, NetId, WorldSchema, WorldUnpacked,
};

pub type DescriptorFetchItem<'a, T> =
    <<<T as Descriptor>::Query as Query>::Fetch as Fetch<'a>>::Item;

pub type DescriptorPackType<T> = <T as Descriptor>::Pack;

pub trait Descriptor: 'static {
    type Query: Query;

    /// Data ready to be unpacked.
    type Pack: serde::de::DeserializeOwned;

    fn modify(
        pack: DescriptorPackType<Self>,
        item: DescriptorFetchItem<'_, Self>,
        entity: Entity,
        spawner: &mut Spawner,
    );
    fn insert(
        pack: DescriptorPackType<Self>,
        entity: Entity,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
    );
    fn on_remove(item: DescriptorFetchItem<'_, Self>, entity: Entity, spawner: &mut Spawner);
    fn remove(entity: Entity, world: &mut World);
}

pub trait SelfDescriptor: Component + serde::de::DeserializeOwned {
    /// Get descriptor
    fn descriptor() -> PhantomData<Self> {
        PhantomData
    }

    fn modify(&mut self, new: Self, entity: Entity, spawner: &mut Spawner);
    fn insert(self, entity: Entity, world: &mut World, res: &mut Res, spawner: &mut Spawner);
    fn on_remove(&mut self, entity: Entity, spawner: &mut Spawner);
}

impl<T> Descriptor for PhantomData<T>
where
    T: SelfDescriptor,
{
    type Query = &'static mut T;
    type Pack = T;

    #[inline(always)]
    fn modify(pack: T, item: &mut T, entity: Entity, spawner: &mut Spawner) {
        item.modify(pack, entity, spawner)
    }

    #[inline(always)]
    fn insert(pack: T, entity: Entity, world: &mut World, res: &mut Res, spawner: &mut Spawner) {
        pack.insert(entity, world, res, spawner)
    }

    #[inline(always)]
    fn on_remove(item: &mut T, entity: Entity, spawner: &mut Spawner) {
        item.on_remove(entity, spawner);
    }

    #[inline(always)]
    fn remove(entity: Entity, world: &mut World) {
        let _ = world.remove_one::<T>(entity);
    }
}

pub trait TrivialDescriptor: Component + serde::de::DeserializeOwned + Clone + PartialEq {}

impl<T> SelfDescriptor for T
where
    T: TrivialDescriptor,
{
    #[inline(always)]
    fn modify(&mut self, new: Self, _entity: Entity, _spawner: &mut Spawner) {
        *self = new;
    }

    #[inline(always)]
    fn insert(self, entity: Entity, world: &mut World, _res: &mut Res, _spawner: &mut Spawner) {
        world.insert_one(entity, self).unwrap();
    }

    #[inline(always)]
    fn on_remove(&mut self, _entity: Entity, _spawner: &mut Spawner) {}
}

struct PlayerIdDescriptor {}

impl Descriptor for PlayerIdDescriptor {
    type Query = &'static mut PlayerId;
    type Pack = NonZeroU64;

    fn modify(pack: NonZeroU64, item: &mut PlayerId, _entity: Entity, _spawner: &mut Spawner) {
        item.0 = pack;
    }
    fn insert(
        pack: NonZeroU64,
        entity: Entity,
        world: &mut World,
        _res: &mut Res,
        _spawner: &mut Spawner,
    ) {
        world.insert_one(entity, PlayerId(pack)).unwrap();
    }
    fn on_remove(_item: &mut PlayerId, _entity: Entity, _spawner: &mut Spawner) {}
    fn remove(entity: Entity, world: &mut World) {
        world.remove_one::<PlayerId>(entity).unwrap();
    }
}

trait Replicator {
    fn replicate<B>(
        unpacked: WorldUnpacked<'_>,
        mapper: &mut EntityMapper,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
        scope: &Scope<'_>,
    ) -> Result<(), BadPacked>
    where
        B: BitTest + serde::de::DeserializeOwned;
}

enum TrackReplicate {
    Unmodified,
    Modified,
    Removed,
}

#[derive(Debug, thiserror::Error)]
pub enum BadPacked {
    #[error("Invalid entity mask")]
    InvalidMask,

    #[error("Invalid bincode")]
    InvalidBincode,
}

impl TrackReplicate {
    pub fn from_mask<'a, B>(idx: usize, mask: &B) -> Result<Self, BadPacked>
    where
        B: BitTest,
    {
        match (mask.test(idx * 2), mask.test(idx * 2 + 1)) {
            (false, false) => Ok(TrackReplicate::Unmodified),
            (true, true) => Ok(TrackReplicate::Modified),
            (true, false) => Ok(TrackReplicate::Removed),
            (false, true) => Err(BadPacked::InvalidMask),
        }
    }
}

fn replicate_one<'a, T, O>(
    track: TrackReplicate,
    entity: Entity,
    world: &mut World,
    res: &mut Res,
    spawner: &mut Spawner,
    opts: O,
    cursor: &'a mut Cursor<&[u8]>,
) -> Result<(), BadPacked>
where
    O: bincode::Options,
    T: Descriptor,
{
    let item = world.query_one_mut::<T::Query>(entity).ok();
    match (item.is_some(), track) {
        (false, TrackReplicate::Removed | TrackReplicate::Unmodified) => {
            drop(item);
        }
        (true, TrackReplicate::Unmodified) => {
            drop(item);
        }
        (true, TrackReplicate::Removed) => {
            T::on_remove(item.unwrap(), entity, spawner);
            T::remove(entity, world);
        }
        (false, TrackReplicate::Modified) => {
            drop(item);
            let pack = opts.deserialize_from(cursor).map_err(|err| {
                tracing::error!("Error deserializing bincode: {}", err);
                BadPacked::InvalidBincode
            })?;
            T::insert(pack, entity, world, res, spawner);
        }
        (true, TrackReplicate::Modified) => {
            let pack = opts.deserialize_from(cursor).map_err(|err| {
                tracing::error!("Error deserializing bincode: {}", err);
                BadPacked::InvalidBincode
            })?;
            T::modify(pack, item.unwrap(), entity, spawner)
        }
    }
    Ok(())
}

impl Replicator for () {
    fn replicate<B>(
        unpacked: WorldUnpacked<'_>,
        mapper: &mut EntityMapper,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
        _scope: &Scope<'_>,
    ) -> Result<(), BadPacked>
    where
        B: BitTest + serde::de::DeserializeOwned,
    {
        let opts = bincode::DefaultOptions::new().allow_trailing_bytes();

        let mut cursor = Cursor::new(unpacked.raw);

        for _ in 0..unpacked.updated {
            let header: EntityHeader<B> = opts.deserialize_from(&mut cursor).map_err(|err| {
                tracing::error!("Error deserializing bincode: {}", err);
                BadPacked::InvalidBincode
            })?;

            let entity = mapper.get_or_spawn(world, header.nid);

            // Begin for each component + player_id.

            let pid = TrackReplicate::from_mask(0, &header.mask)?;
            replicate_one::<PlayerIdDescriptor, _>(
                pid,
                entity,
                world,
                res,
                spawner,
                opts,
                &mut cursor,
            )?;

            // End for each component + player_id.
        }

        for _ in 0..unpacked.removed {
            let nid: NetId = opts.deserialize_from(&mut cursor).map_err(|err| {
                tracing::error!("Error deserializing bincode: {}", err);
                BadPacked::InvalidBincode
            })?;

            if let Some(entity) = mapper.get(nid) {
                // Begin for each component.
                // End for each component.

                let _ = world.despawn(entity);
            }
        }

        Ok(())
    }
}

pub struct ServerStep {
    pub value: u64,
}

pub struct ClientBuilder<R> {
    ids: Vec<TypeId>,
    marker: PhantomData<fn(R)>,
}

impl ClientBuilder<()> {
    pub fn new() -> Self {
        ClientBuilder {
            ids: Vec::new(),
            marker: PhantomData,
        }
    }

    pub fn with<T>(mut self, descriptor: T) -> ClientBuilder<(T,)>
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
        ClientBuilder {
            ids: self.ids,
            marker: PhantomData,
        }
    }

    pub fn build<I>(&self) -> ClientSystem
    where
        I: Component + serde::Serialize,
    {
        ClientSystem::new::<I, ()>()
    }
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
    pub fn builder() -> ClientBuilder<()> {
        ClientBuilder::new()
    }

    fn new<I, R>() -> Self
    where
        I: Component + serde::Serialize,
        R: Replicator,
    {
        ClientSystem {
            session: None,
            mapper: EntityMapper::new(),
            controlled: HashSet::new(),
            send_inputs: send_inputs::<I>,
            replicate: replicate::<R, u32>,
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

    pub async fn add_player(
        &mut self,
        player: &impl serde::Serialize,
        scope: &Scope<'_>,
    ) -> eyre::Result<PlayerId> {
        let opts = bincode::DefaultOptions::new().allow_trailing_bytes();
        let player = opts.serialize(player)?;

        let id = self
            .session
            .as_mut()
            .expect("Attempt to add player in disconnected ClientSystem")
            .add_player::<Bytes, PlayerId, _>(player, scope)
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

struct InputPack<'a, T> {
    queue: &'a mut CommandQueue<T>,
}

impl<'a, T> Pack<InputSchema> for InputPack<'a, T>
where
    T: serde::Serialize,
{
    fn pack(self, offset: usize, output: &mut [u8]) -> (InputPacked, usize) {
        let opts = bincode::DefaultOptions::new().allow_trailing_bytes();
        let mut cursor = Cursor::new(output);
        for command in self.queue.drain() {
            opts.serialize_into(&mut cursor, &command).unwrap();
        }

        let len = cursor.position() as usize;

        (
            InputPacked {
                offset: offset as FixedUsize,
                len: len as FixedUsize,
            },
            len,
        )
    }
}

fn send_inputs<'a, I>(
    controlled: &HashSet<PlayerId>,
    session: &'a mut ClientSession<TcpChannel>,
    cx: SystemContext<'a>,
) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + 'a>>
where
    I: Component + serde::Serialize,
{
    let scope: &'a Scope<'static> = &*cx.scope;

    let inputs = cx
        .world
        .query_mut::<(&PlayerId, &NetId, &mut CommandQueue<I>)>()
        .into_iter()
        .filter_map(|(_, (pid, nid, queue))| {
            controlled
                .contains(pid)
                .then(move || (*pid, (*nid, InputPack { queue })))
        });

    let mut vec_inputs = Vec::new_in(scope);

    vec_inputs.extend(inputs);

    tracing::debug!("Sending input ({})", session.current_step());
    Box::pin(async move {
        session
            .send_inputs::<(NetId, InputSchema), _, _>(vec_inputs, scope)
            .await
            .wrap_err("Failed to send inputs from client to server")
    })
}

fn replicate<R, B>(
    session: &mut ClientSession<TcpChannel>,
    mapper: &mut EntityMapper,
    cx: SystemContext<'_>,
) -> eyre::Result<()>
where
    R: Replicator,
    B: BitTest + serde::de::DeserializeOwned,
{
    let updates = session.advance::<WorldSchema>(cx.scope)?;
    if let Some(updates) = updates {
        tracing::debug!("Received updates ({})", updates.server_step);

        cx.res.with(|| ServerStep { value: 0 }).value = updates.server_step;

        R::replicate::<B>(
            updates.updates,
            mapper,
            cx.world,
            cx.res,
            cx.spawner,
            cx.scope,
        )?;
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

macro_rules! for_tuple {
    ($($t:ident $b:ident),+ $(,)?) => {
        impl<$( $t ),+> ClientBuilder<($( $t, )+)> {
            pub fn with<T>(mut self, descriptor: T) -> ClientBuilder<($($t,)+ T,)>
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
                ClientBuilder {
                    ids: self.ids,
                    marker: PhantomData,
                }
            }

            pub fn build<INPUT>(&self) -> ClientSystem
            where
                INPUT: Component + serde::Serialize,
                $(
                    $t: Descriptor,
                )+
            {
                ClientSystem::new::<INPUT, ($($t,)+)>()
            }
        }

        impl<$( $t ),+> Replicator for ($( $t, )+)
        where
            $(
                $t: Descriptor,
            )+
        {
            fn replicate<BITSET>(
                unpacked: WorldUnpacked<'_>,
                mapper: &mut EntityMapper,
                world: &mut World,
                res: &mut Res,
                spawner: &mut Spawner,
                _scope: &Scope<'_>,
            ) -> Result<(), BadPacked>
            where
                BITSET: BitTest + serde::de::DeserializeOwned,
            {
                let opts = bincode::DefaultOptions::new().allow_trailing_bytes();

                let mut cursor = Cursor::new(unpacked.raw);

                for _ in 0..unpacked.updated {
                    // tracing::error!("REST: {:?}", &cursor.get_ref()[cursor.position() as usize..]);

                    let header: EntityHeader<BITSET> = opts
                        .deserialize_from(&mut cursor)
                        .map_err(|_| BadPacked::InvalidBincode)?;

                    // tracing::error!("NetId: {:?}", header.nid);

                    let entity = mapper.get_or_spawn(world, header.nid);

                    let mut mask_idx = 0;

                    // Begin for each component + player_id.

                    $(
                        let $b = TrackReplicate::from_mask(mask_idx, &header.mask)?;
                        replicate_one::<$t, _>($b, entity, world, res, spawner, opts, &mut cursor)?;
                        mask_idx += 1;
                    )+

                    let pid = TrackReplicate::from_mask(mask_idx, &header.mask)?;
                    replicate_one::<PlayerIdDescriptor, _>(pid, entity, world, res, spawner, opts, &mut cursor)?;

                    // End for each component + player_id.
                }

                for _ in 0..unpacked.removed {
                    let nid: NetId = opts.deserialize_from(&mut cursor).map_err(|err| {
                        tracing::error!("Error deserializing bincode: {}", err);
                        BadPacked::InvalidBincode
                    })?;

                    if let Some(entity) = mapper.get(nid) {
                        {
                            let ($( $b, )+) = world
                                .query_one_mut::<($( Option<<$t as Descriptor>::Query>, )+)>(entity)
                                .unwrap();

                            // Begin for each component.

                            $(
                                if let Some($b) = $b {
                                    <$t as Descriptor>::on_remove($b, entity, spawner)
                                }
                            )+

                            // End for each component.
                        }

                        let _ = world.despawn(entity);
                    }
                }

                Ok(())
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
