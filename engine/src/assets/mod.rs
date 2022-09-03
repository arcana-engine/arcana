//! Asset loading facility.

mod cache;

#[cfg(feature = "asset-pipeline")]
pub mod treasury;

#[cfg(feature = "asset-pipeline")]
pub mod import;

#[cfg(feature = "graphics")]
pub mod image;

use std::{
    any::TypeId,
    borrow::Borrow,
    convert::Infallible,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

pub use goods::{
    Asset, AssetBuild, AssetField, AssetFieldBuild, AssetHandle, AssetId, AssetLookup, AssetResult,
    Error, External, Key, Loader, TrivialAsset, TypedAssetId,
};
use hashbrown::hash_map::{Entry, HashMap};

use crate::noophash::NoopHasherBuilder;

use self::cache::{AnyAssetCache, AssetCache};

// #[cfg(feature = "visible")]
// pub use self::{
//     font::{
//         FontAsset, FontFaces, FontFacesBuildError, FontFacesCache, FontFacesDecodeError,
//         FontFacesDecoded,
//     },
//     image::ImageAsset,
//     sprite_sheet::{SpriteAnimation, SpriteFrame, SpriteRect, SpriteSheet, SpriteSize},
// };

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
        ExternAssetWithIdFut(loader.load(id), id)
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

        match Pin::new(&mut me.0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => Poll::Ready(Ok(WithId {
                asset: result,
                id: me.1,
            })),
        }
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

pub trait TypedAssetIdExt: Borrow<AssetId> {
    type Asset: Asset;
}

impl<A> TypedAssetIdExt for TypedAssetId<A>
where
    A: Asset,
{
    type Asset = A;
}

/// Sync asset loader.
pub struct Assets {
    pub loader: Loader,
    caches: HashMap<TypeId, Box<dyn AnyAssetCache>, NoopHasherBuilder>,
}

impl Assets {
    pub fn new(loader: Loader) -> Self {
        Assets {
            loader,
            caches: HashMap::with_hasher(NoopHasherBuilder),
        }
    }

    pub fn cleanup(&mut self) {
        self.caches.values_mut().for_each(|cache| cache.cleanup());
    }

    pub fn build<A, B>(&mut self, id: AssetId, builder: &mut B) -> Option<Result<&A, &Error>>
    where
        A: AssetBuild<B>,
    {
        let cache = match self.caches.entry(TypeId::of::<A>()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(Box::new(AssetCache::<A>::new())),
        };
        cache.cast::<A>().build(id, &self.loader, builder)
    }

    pub fn get<A>(&mut self, id: AssetId) -> Option<Result<&A, &Error>>
    where
        A: TrivialAsset,
    {
        let cache = match self.caches.entry(TypeId::of::<A>()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(Box::new(AssetCache::<A>::new())),
        };
        cache.cast::<A>().build(id, &self.loader, &mut ())
    }

    pub fn load<'a, A, K>(&mut self, key: K) -> AssetHandle<A>
    where
        A: Asset,
        K: Into<Key<'a>>,
    {
        self.loader.load::<A, K>(key)
    }

    pub fn lookup<A>(&mut self, key: &str) -> AssetLookup
    where
        A: Asset,
    {
        self.loader.lookup::<A>(key)
    }

    pub async fn get_async<'a, A, K>(&mut self, key: K) -> Result<A, Error>
    where
        A: TrivialAsset,
        K: Into<Key<'a>>,
    {
        let handle = self.loader.load::<A, K>(key);
        let mut result = handle.await;
        Ok(result.get()?.clone())
    }

    pub async fn build_async<'a, A, B, K>(&mut self, key: K, builder: &mut B) -> Result<A, Error>
    where
        A: AssetBuild<B>,
        K: Into<Key<'a>>,
    {
        let handle = self.loader.load::<A, K>(key);
        let mut result = handle.await;
        Ok(result.build(builder)?.clone())
    }
}
