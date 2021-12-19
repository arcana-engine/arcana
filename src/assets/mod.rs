//! Asset loading facility.

mod cache;

#[cfg(feature = "visible")]
mod image;
#[cfg(all(feature = "visible", feature = "3d"))]
mod object;
#[cfg(feature = "visible")]
mod sprite_sheet;

#[cfg(feature = "visible")]
mod font;

use std::{
    borrow::Borrow,
    convert::Infallible,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{ready, Context, Poll},
};

use goods::TypedAssetId;
pub use goods::{
    Asset, AssetBuild, AssetField, AssetFieldBuild, AssetHandle, AssetId, AssetResult, Error,
    External, Loader,
};

pub use self::cache::{AssetLoadCache, AssetLoadCacheClearSystem};

#[cfg(feature = "visible")]
pub use self::{
    font::{
        FontAsset, FontFaces, FontFacesBuildError, FontFacesCache, FontFacesDecodeError,
        FontFacesDecoded,
    },
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

pub trait TypedAssetIdExt: Borrow<AssetId> {
    type Asset: Asset;
}

impl<A> TypedAssetIdExt for TypedAssetId<A>
where
    A: Asset,
{
    type Asset = A;
}
