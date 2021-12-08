//! Asset loading facility.

#[cfg(feature = "visible")]
pub mod image;
#[cfg(all(feature = "visible", feature = "3d"))]
pub mod object;
#[cfg(feature = "visible")]
pub mod sprite_sheet;

#[cfg(feature = "visible")]
pub mod font;

use std::{
    collections::hash_map::{Entry, HashMap},
    convert::Infallible,
    future::Future,
    mem::swap,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{ready, Context, Poll},
};

pub use goods::{
    Asset, AssetBuild, AssetField, AssetFieldBuild, AssetHandle, AssetId, AssetResult, Error,
    External, Loader,
};

use crate::{with_async_task_context, Spawner, TaskContext};

#[cfg(feature = "visible")]
pub use self::{
    image::ImageAsset,
    sprite_sheet::{SpriteAnimation, SpriteFrame, SpriteRect, SpriteSheet, SpriteSize},
};

#[cfg(not(feature = "visible"))]
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WithId<A> {
    asset: A,
    id: AssetId,
}

impl<A> Deref for WithId<A> {
    type Target = A;

    #[inline(always)]
    fn deref(&self) -> &A {
        &self.asset
    }
}

impl<A> DerefMut for WithId<A> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.asset
    }
}

impl<A> WithId<A> {
    #[inline(always)]
    pub fn new(asset: A, id: AssetId) -> Self {
        WithId { asset, id }
    }

    #[inline(always)]
    pub fn asset(with_id: Self) -> A {
        with_id.asset
    }

    #[inline(always)]
    pub fn asset_ref(with_id: &Self) -> &A {
        &with_id.asset
    }

    #[inline(always)]
    pub fn asset_mut(with_id: &mut Self) -> &mut A {
        &mut with_id.asset
    }

    #[inline(always)]
    pub fn id(with_id: &Self) -> AssetId {
        with_id.id
    }
}

impl<A> AssetField<External> for WithId<A>
where
    A: Asset,
{
    type Info = AssetId;
    type DecodeError = Infallible;
    type BuildError = Error;
    type Decoded = WithId<AssetResult<A>>;
    type Fut = ExternAssetWithIdFut<A>;

    #[inline(always)]
    fn decode(id: AssetId, loader: &Loader) -> Self::Fut {
        ExternAssetWithIdFut(loader.load(&id), id)
    }
}

pub struct ExternAssetWithIdFut<A>(AssetHandle<A>, AssetId);

impl<A> Future for ExternAssetWithIdFut<A>
where
    A: Asset,
{
    type Output = Result<WithId<AssetResult<A>>, Infallible>;

    #[inline(always)]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();

        let result = ready!(Pin::new(&mut me.0).poll(cx));
        Poll::Ready(Ok(WithId {
            asset: result,
            id: me.1,
        }))
    }
}

impl<A, B> AssetFieldBuild<External, B> for WithId<A>
where
    A: Asset + AssetBuild<B>,
{
    #[inline(always)]
    fn build(mut result: WithId<AssetResult<A>>, builder: &mut B) -> Result<WithId<A>, Error> {
        Ok(WithId {
            asset: result.asset.build(builder)?.clone(),
            id: result.id,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(not(feature = "visible"), repr(transparent))]
pub struct WithIdVisible<A> {
    id: AssetId,
    #[cfg(feature = "visible")]
    asset: A,
    #[cfg(not(feature = "visible"))]
    marker: PhantomData<A>,
}

#[cfg(feature = "visible")]
impl<A> Deref for WithIdVisible<A> {
    type Target = A;

    #[inline(always)]
    fn deref(&self) -> &A {
        &self.asset
    }
}

#[cfg(feature = "visible")]
impl<A> DerefMut for WithIdVisible<A> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.asset
    }
}

impl<A> WithIdVisible<A> {
    #[cfg(feature = "visible")]
    #[inline(always)]
    pub fn new(id: AssetId, asset: A) -> Self {
        WithIdVisible { id, asset }
    }

    #[cfg(not(feature = "visible"))]
    #[inline(always)]
    pub fn new(id: AssetId) -> Self {
        WithIdVisible {
            id,
            marker: PhantomData,
        }
    }

    #[cfg(feature = "visible")]
    #[inline(always)]
    pub fn asset(with_id: Self) -> A {
        with_id.asset
    }

    #[cfg(feature = "visible")]
    #[inline(always)]
    pub fn asset_ref(with_id: &Self) -> &A {
        &with_id.asset
    }

    #[cfg(feature = "visible")]
    #[inline(always)]
    pub fn asset_mut(with_id: &mut Self) -> &mut A {
        &mut with_id.asset
    }

    #[inline(always)]
    pub fn id(with_id: &Self) -> AssetId {
        with_id.id
    }
}

impl<A> AssetField<External> for WithIdVisible<A>
where
    A: Asset,
{
    type Info = AssetId;
    type DecodeError = Infallible;
    type BuildError = Error;
    type Decoded = WithIdVisible<AssetResult<A>>;
    type Fut = ExternAssetWithIdVisibleFut<A>;

    #[inline(always)]
    fn decode(id: AssetId, loader: &Loader) -> Self::Fut {
        ExternAssetWithIdVisibleFut {
            id,
            #[cfg(feature = "visible")]
            asset: loader.load(&id),
            #[cfg(not(feature = "visible"))]
            marker: PhantomData,
        }
    }
}

pub struct ExternAssetWithIdVisibleFut<A> {
    id: AssetId,
    #[cfg(feature = "visible")]
    asset: AssetHandle<A>,

    #[cfg(not(feature = "visible"))]
    marker: PhantomData<AssetHandle<A>>,
}

impl<A> Future for ExternAssetWithIdVisibleFut<A>
where
    A: Asset,
{
    type Output = Result<WithIdVisible<AssetResult<A>>, Infallible>;

    #[inline(always)]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();

        #[cfg(feature = "visible")]
        let result = ready!(Pin::new(&mut me.asset).poll(cx));

        Poll::Ready(Ok(WithIdVisible {
            id: me.id,

            #[cfg(feature = "visible")]
            asset: result,

            #[cfg(not(feature = "visible"))]
            marker: PhantomData,
        }))
    }
}

#[cfg(feature = "visible")]
impl<A, B> AssetFieldBuild<External, B> for WithIdVisible<A>
where
    A: Asset + AssetBuild<B>,
{
    #[inline(always)]
    fn build(
        mut result: WithIdVisible<AssetResult<A>>,
        builder: &mut B,
    ) -> Result<WithIdVisible<A>, Error> {
        Ok(WithIdVisible {
            id: result.id,
            asset: result.asset.build(builder)?.clone(),
        })
    }
}

#[cfg(feature = "visible")]
impl<A> From<WithIdVisible<A>> for WithId<A> {
    fn from(value: WithIdVisible<A>) -> Self {
        WithId {
            asset: value.asset,
            id: value.id,
        }
    }
}

impl<A> From<WithId<A>> for WithIdVisible<A> {
    fn from(value: WithId<A>) -> Self {
        WithIdVisible {
            id: value.id,
            #[cfg(feature = "visible")]
            asset: value.asset,
            #[cfg(not(feature = "visible"))]
            marker: PhantomData,
        }
    }
}

impl<A> From<WithId<A>> for AssetId {
    fn from(value: WithId<A>) -> Self {
        value.id
    }
}

impl<A> From<WithIdVisible<A>> for AssetId {
    fn from(value: WithIdVisible<A>) -> Self {
        value.id
    }
}

pub struct AssetLoadSystemCache<A: Asset> {
    to_load: Vec<(AssetId, Option<AssetHandle<A>>, Option<AssetResult<A>>)>,
    loaded: HashMap<AssetId, Option<A>>,
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

    pub fn ensure_load(&mut self, id: AssetId, loader: &Loader) {
        match self.loaded.entry(id) {
            Entry::Occupied(_) => return,
            Entry::Vacant(entry) => {
                let handle = loader.load::<A>(&id);
                entry.insert(None);
                self.to_load.push((id, Some(handle), None));
            }
        }
    }

    pub fn get_ready(&self, id: &AssetId) -> Option<&A> {
        self.loaded.get(id).and_then(Option::as_ref)
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
                        for (id, handle, result) in to_load.drain(..) {
                            debug_assert!(result.is_some());
                            debug_assert!(handle.is_none());

                            let mut result = result.unwrap();
                            let set = result.build(builder(cx.reborrow()));

                            match set {
                                Ok(set) => {
                                    let me = cx.res.get_mut::<Self>().unwrap();
                                    me.loaded.insert(id, Some(set.clone()));
                                }
                                Err(err) => {
                                    tracing::error!("Failed to load set '{}': {:#}", id, err);
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
