//! Asset loading facility.

mod asset;
mod dataurl;
mod format;
mod fs;
mod key;
mod loader;
mod source;

pub mod gltf;
pub mod image;
pub mod material;

pub use self::{
    dataurl::DataUrlSource,
    fs::FsSource,
    image::ImageAsset,
    loader::{AssetHandle, AssetResult, AssetResultPoisoned, Error, Loader, LoaderBuilder},
    source::Source,
};

#[derive(Debug, thiserror::Error)]
#[error("Not found")]
struct NotFound;
