use std::error::Error;

pub struct PhantomBuilder;

/// An asset type that can be built from decoded representation.
pub trait Asset: Clone + Sized + Send + Sync + 'static {
    /// Error building asset instance from decoded representation.
    type Error: Error + Send + Sync + 'static;

    /// Decoded representation of this asset.
    type Decoded: Send + Sync;

    /// Builder required to build asset from decoded value.
    type Builder;

    /// Build asset instance using decoded representation and `Resources`.
    fn build(decoded: Self::Decoded, builder: &mut Self::Builder) -> Result<Self, Self::Error>;
}
/// Simple asset that does not require building.
pub trait SimpleAsset: Clone + Sized + Send + Sync + 'static {}

impl<A> Asset for A
where
    A: SimpleAsset,
{
    type Error = std::convert::Infallible;
    type Decoded = Self;
    type Builder = PhantomBuilder;

    fn build(decoded: Self, _: &mut PhantomBuilder) -> Result<Self, std::convert::Infallible> {
        Ok(decoded)
    }
}
