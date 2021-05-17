use arcana::graphics::*;

#[descriptors]
pub struct RasterDescriptors {
    #[sampler]
    sampler: Sampler,

    #[sampled_image]
    textures: [ImageView; 128],
}
