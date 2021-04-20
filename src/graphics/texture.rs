pub use sierra::{ImageView, Sampler};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Texture {
    /// Image view of the loaded texture.
    pub image: ImageView,

    /// Sampler associated with the texture image.
    pub sampler: Sampler,
}
