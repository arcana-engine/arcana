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
    convert::Infallible,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{ready, Context, Poll},
};

use goods::{
    Asset, AssetBuild, AssetField, AssetFieldBuild, AssetHandle, AssetResult, Error, External,
    Loader,
};
use uuid::Uuid;

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
    fn deref(&self) -> &A {
        &self.asset
    }
}

impl<A> DerefMut for WithUuid<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.asset
    }
}

impl<A> WithUuid<A> {
    pub fn new(asset: A, uuid: Uuid) -> Self {
        WithUuid { asset, uuid }
    }

    pub fn asset(with_uuid: Self) -> A {
        with_uuid.asset
    }

    pub fn asset_ref(with_uuid: &Self) -> &A {
        &with_uuid.asset
    }

    pub fn asset_mut(with_uuid: &mut Self) -> &mut A {
        &mut with_uuid.asset
    }

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
    fn build(mut result: WithUuid<AssetResult<A>>, builder: &mut B) -> Result<WithUuid<A>, Error> {
        Ok(WithUuid {
            asset: result.asset.get(builder)?.clone(),
            uuid: result.uuid,
        })
    }
}

#[cfg(feature = "server")]
pub type WithUuidOnServer<A> = WithUuid<A>;

#[cfg(not(feature = "server"))]
pub type WithUuidOnServer<A> = A;
