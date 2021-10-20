//! Asset loading facility.

#[cfg(feature = "visible")]
pub mod image;
#[cfg(all(feature = "visible", feature = "3d"))]
pub mod object;
#[cfg(feature = "visible")]
pub mod sprite_sheet;

#[cfg(feature = "physics2d")]
pub mod tiles;

use std::{
    collections::hash_map::{Entry, HashMap},
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    mem::swap,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{ready, Context, Poll},
};

use goods::{
    Asset, AssetBuild, AssetField, AssetFieldBuild, AssetHandle, AssetResult, Error, External,
    Loader,
};
use uuid::Uuid;

use crate::{with_async_task_context, Spawner, TaskContext};

#[cfg(feature = "visible")]
pub use self::{
    image::ImageAsset,
    sprite_sheet::{SpriteAnimation, SpriteFrame, SpriteRect, SpriteSheet, SpriteSize},
};

#[cfg(feature = "physics2d")]
pub use self::tiles::{Tile, TileMap, TileSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WithUuid<A> {
    asset: A,
    uuid: Uuid,
}

impl<A> Deref for WithUuid<A> {
    type Target = A;

    #[inline(always)]
    fn deref(&self) -> &A {
        &self.asset
    }
}

impl<A> DerefMut for WithUuid<A> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.asset
    }
}

impl<A> WithUuid<A> {
    #[inline(always)]
    pub fn new(asset: A, uuid: Uuid) -> Self {
        WithUuid { asset, uuid }
    }

    #[inline(always)]
    pub fn asset(with_uuid: Self) -> A {
        with_uuid.asset
    }

    #[inline(always)]
    pub fn asset_ref(with_uuid: &Self) -> &A {
        &with_uuid.asset
    }

    #[inline(always)]
    pub fn asset_mut(with_uuid: &mut Self) -> &mut A {
        &mut with_uuid.asset
    }

    #[inline(always)]
    pub fn uuid(with_uuid: &Self) -> Uuid {
        with_uuid.uuid
    }
}

impl<A> AssetField<External> for WithUuid<A>
where
    A: Asset,
{
    type Info = Uuid;
    type DecodeError = Infallible;
    type BuildError = Error;
    type Decoded = WithUuid<AssetResult<A>>;
    type Fut = ExternAssetWithUuidFut<A>;

    #[inline(always)]
    fn decode(uuid: Uuid, loader: &Loader) -> Self::Fut {
        ExternAssetWithUuidFut(loader.load(&uuid), uuid)
    }
}

pub struct ExternAssetWithUuidFut<A>(AssetHandle<A>, Uuid);

impl<A> Future for ExternAssetWithUuidFut<A>
where
    A: Asset,
{
    type Output = Result<WithUuid<AssetResult<A>>, Infallible>;

    #[inline(always)]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();

        let result = ready!(Pin::new(&mut me.0).poll(cx));
        Poll::Ready(Ok(WithUuid {
            asset: result,
            uuid: me.1,
        }))
    }
}

impl<A, B> AssetFieldBuild<External, B> for WithUuid<A>
where
    A: Asset + AssetBuild<B>,
{
    #[inline(always)]
    fn build(mut result: WithUuid<AssetResult<A>>, builder: &mut B) -> Result<WithUuid<A>, Error> {
        Ok(WithUuid {
            asset: result.asset.get(builder)?.clone(),
            uuid: result.uuid,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(not(feature = "visible"), repr(transparent))]
pub struct WithUuidVisible<A> {
    uuid: Uuid,
    #[cfg(feature = "visible")]
    asset: A,
    #[cfg(not(feature = "visible"))]
    marker: PhantomData<A>,
}

#[cfg(feature = "visible")]
impl<A> Deref for WithUuidVisible<A> {
    type Target = A;

    #[inline(always)]
    fn deref(&self) -> &A {
        &self.asset
    }
}

#[cfg(feature = "visible")]
impl<A> DerefMut for WithUuidVisible<A> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.asset
    }
}

impl<A> WithUuidVisible<A> {
    #[cfg(feature = "visible")]
    #[inline(always)]
    pub fn new(uuid: Uuid, asset: A) -> Self {
        WithUuidVisible { uuid, asset }
    }

    #[cfg(not(feature = "visible"))]
    #[inline(always)]
    pub fn new(uuid: Uuid) -> Self {
        WithUuidVisible {
            uuid,
            marker: PhantomData,
        }
    }

    #[cfg(feature = "visible")]
    #[inline(always)]
    pub fn asset(with_uuid: Self) -> A {
        with_uuid.asset
    }

    #[cfg(feature = "visible")]
    #[inline(always)]
    pub fn asset_ref(with_uuid: &Self) -> &A {
        &with_uuid.asset
    }

    #[cfg(feature = "visible")]
    #[inline(always)]
    pub fn asset_mut(with_uuid: &mut Self) -> &mut A {
        &mut with_uuid.asset
    }

    #[inline(always)]
    pub fn uuid(with_uuid: &Self) -> Uuid {
        with_uuid.uuid
    }
}

impl<A> AssetField<External> for WithUuidVisible<A>
where
    A: Asset,
{
    type Info = Uuid;
    type DecodeError = Infallible;
    type BuildError = Error;
    type Decoded = WithUuidVisible<AssetResult<A>>;
    type Fut = ExternAssetWithUuidVisibleFut<A>;

    #[inline(always)]
    fn decode(uuid: Uuid, loader: &Loader) -> Self::Fut {
        ExternAssetWithUuidVisibleFut {
            uuid,
            #[cfg(feature = "visible")]
            asset: loader.load(&uuid),
            #[cfg(not(feature = "visible"))]
            marker: PhantomData,
        }
    }
}

pub struct ExternAssetWithUuidVisibleFut<A> {
    uuid: Uuid,
    #[cfg(feature = "visible")]
    asset: AssetHandle<A>,

    #[cfg(not(feature = "visible"))]
    marker: PhantomData<AssetHandle<A>>,
}

impl<A> Future for ExternAssetWithUuidVisibleFut<A>
where
    A: Asset,
{
    type Output = Result<WithUuidVisible<AssetResult<A>>, Infallible>;

    #[inline(always)]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();

        #[cfg(feature = "visible")]
        let result = ready!(Pin::new(&mut me.asset).poll(cx));

        Poll::Ready(Ok(WithUuidVisible {
            uuid: me.uuid,

            #[cfg(feature = "visible")]
            asset: result,

            #[cfg(not(feature = "visible"))]
            marker: PhantomData,
        }))
    }
}

#[cfg(feature = "visible")]
impl<A, B> AssetFieldBuild<External, B> for WithUuidVisible<A>
where
    A: Asset + AssetBuild<B>,
{
    #[inline(always)]
    fn build(
        mut result: WithUuidVisible<AssetResult<A>>,
        builder: &mut B,
    ) -> Result<WithUuidVisible<A>, Error> {
        Ok(WithUuidVisible {
            uuid: result.uuid,

            #[cfg(feature = "visible")]
            asset: result.asset.get(builder)?.clone(),
        })
    }
}

#[cfg(feature = "visible")]
impl<A> From<WithUuidVisible<A>> for WithUuid<A> {
    fn from(value: WithUuidVisible<A>) -> Self {
        WithUuid {
            asset: value.asset,
            uuid: value.uuid,
        }
    }
}

impl<A> From<WithUuid<A>> for WithUuidVisible<A> {
    fn from(value: WithUuid<A>) -> Self {
        WithUuidVisible {
            uuid: value.uuid,
            #[cfg(feature = "visible")]
            asset: value.asset,
            #[cfg(not(feature = "visible"))]
            marker: PhantomData,
        }
    }
}

impl<A> From<WithUuid<A>> for Uuid {
    fn from(value: WithUuid<A>) -> Self {
        value.uuid
    }
}

impl<A> From<WithUuidVisible<A>> for Uuid {
    fn from(value: WithUuidVisible<A>) -> Self {
        value.uuid
    }
}

pub struct AssetLoadSystemCache<A: Asset> {
    to_load: Vec<(Uuid, Option<AssetHandle<A>>, Option<AssetResult<A>>)>,
    loaded: HashMap<Uuid, Option<A>>,
    task_running: bool,
}

impl<A> AssetLoadSystemCache<A>
where
    A: Asset,
{
    pub fn new() -> Self {
        AssetLoadSystemCache {
            to_load: Vec::new(),
            loaded: HashMap::new(),
            task_running: false,
        }
    }

    pub fn ensure_load(&mut self, uuid: Uuid, loader: &Loader) {
        match self.loaded.entry(uuid) {
            Entry::Occupied(_) => return,
            Entry::Vacant(entry) => {
                let handle = loader.load::<A>(&uuid);
                entry.insert(None);
                self.to_load.push((uuid, Some(handle), None));
            }
        }
    }

    pub fn get_ready(&self, uuid: &Uuid) -> Option<&A> {
        self.loaded.get(uuid).and_then(Option::as_ref)
    }

    pub fn ensure_task<B, F>(&mut self, spawner: &mut Spawner, builder: F)
    where
        A: AssetBuild<B>,
        B: 'static,
        F: Fn(TaskContext<'_>) -> &mut B + Send + 'static,
    {
        if !self.to_load.is_empty() && !self.task_running {
            self.task_running = true;

            spawner.spawn(async move {
                let mut to_load = Vec::new();

                loop {
                    debug_assert!(to_load.is_empty());

                    let run = with_async_task_context(|cx| {
                        let me = cx.res.get_mut::<Self>().unwrap();

                        if me.to_load.is_empty() {
                            // If there's noting to load - end task.
                            me.task_running = false;
                            return false;
                        }

                        // Or take all sets to load into async scope.
                        swap(&mut me.to_load, &mut to_load);
                        true
                    });

                    if !run {
                        break;
                    }

                    // Ensure all map assets are loaded.
                    for (_, handle, result) in &mut to_load {
                        debug_assert!(handle.is_some());
                        debug_assert!(result.is_none());
                        *result = Some(handle.take().unwrap().await);
                    }

                    with_async_task_context(|mut cx| {
                        for (uuid, handle, result) in to_load.drain(..) {
                            debug_assert!(result.is_some());
                            debug_assert!(handle.is_none());

                            let mut result = result.unwrap();
                            let set = result.get(builder(cx.reborrow()));

                            match set {
                                Ok(set) => {
                                    let me = cx.res.get_mut::<Self>().unwrap();
                                    me.loaded.insert(uuid, Some(set.clone()));
                                }
                                Err(err) => {
                                    tracing::error!("Failed to load set '{}': {:#}", uuid, err);
                                }
                            }
                        }
                    });
                }

                Ok(())
            });
        }
    }

    pub fn clear_ready(&mut self) {
        self.loaded.retain(|_, opt| opt.is_none());
    }
}
