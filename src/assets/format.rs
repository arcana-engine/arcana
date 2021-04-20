use {
    super::{asset::Asset, loader::Loader},
    std::{error::Error, future::Future},
};

/// Data required to decode asset from bytes.
///
/// Examples would be [`PngImage`] format to load png image and
/// `BmpImage` to load bitmap image.
/// Even though both are first loaded as raw bytes.
pub trait Format<A: Asset>: std::fmt::Debug + Send + 'static {
    /// Error that can occur during asset decoding.
    type Error: Error + Send + Sync + 'static;

    /// Future that will resolve into decoded asset when ready.
    type Fut: Future<Output = Result<A::Decoded, Self::Error>> + Send;

    /// Decode asset from bytes loaded from asset source.
    fn decode(self, bytes: Box<[u8]>, key: &str, loader: Loader) -> Self::Fut;
}

/// Trait implemented for assets for which
/// default format can be chosen and constructed out of thin air.
///
/// Allows using `Assets::load` method.
pub trait AssetDefaultFormat: Asset {
    /// Default format type that can be default-constructed.
    type DefaultFormat: Format<Self> + Default;
}
