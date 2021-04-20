use {
    std::{error::Error, future::Future},
    url::Url,
};

/// Asset data loaded from [`Source`].
pub struct AssetData {
    /// Serialized asset data.
    pub bytes: Box<[u8]>,

    /// Opaque version for asset.
    /// It can only by interpreted by [`Source`]
    /// that returned this [`AssetData`] instance.
    pub version: u64,
}

/// Abstract source for asset raw data.
pub trait Source: Send + Sync + 'static {
    /// Error that may occur during asset loading.
    type Error: Error + Send + Sync;

    /// Future that resolves int loaded asset data, error or None.
    type Fut: Future<Output = Result<Option<AssetData>, Self::Error>> + Send;

    /// Load asset data from this source.
    /// Returns `Ok(Some(asset_data))` if asset is loaded successfully.
    /// Returns `Ok(None)` if asset is not found, allowing checking other sources.
    fn load(&self, key: &str) -> Self::Fut;

    /// Update asset data if newer is available.
    fn update(&self, key: &str, version: u64) -> Self::Fut;
}
