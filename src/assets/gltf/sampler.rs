use {
    super::GltfBuildContext,
    crate::graphics::Graphics,
    gltf::texture::{MagFilter, MinFilter, WrappingMode},
    sierra::{
        BorderColor, Filter, MipmapMode, OutOfMemory, Sampler, SamplerAddressMode, SamplerInfo,
    },
    std::collections::hash_map::Entry,
};

impl GltfBuildContext<'_> {
    pub fn get_sampler(&mut self, sampler: gltf::texture::Sampler) -> Result<Sampler, OutOfMemory> {
        match self.samplers.entry(sampler.index()) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => Ok(entry
                .insert(create_sampler(&mut self.graphics, sampler)?)
                .clone()),
        }
    }
}

fn create_sampler(
    graphics: &mut Graphics,
    sampler: gltf::texture::Sampler,
) -> Result<Sampler, OutOfMemory> {
    graphics.create_sampler(SamplerInfo {
        mag_filter: match sampler.mag_filter() {
            None | Some(MagFilter::Nearest) => Filter::Nearest,
            Some(MagFilter::Linear) => Filter::Linear,
        },
        min_filter: match sampler.min_filter() {
            None
            | Some(MinFilter::Nearest)
            | Some(MinFilter::NearestMipmapNearest)
            | Some(MinFilter::NearestMipmapLinear) => Filter::Nearest,
            _ => Filter::Linear,
        },
        mipmap_mode: match sampler.min_filter() {
            None
            | Some(MinFilter::Nearest)
            | Some(MinFilter::Linear)
            | Some(MinFilter::NearestMipmapNearest)
            | Some(MinFilter::LinearMipmapNearest) => MipmapMode::Nearest,
            _ => MipmapMode::Linear,
        },
        address_mode_u: match sampler.wrap_s() {
            WrappingMode::ClampToEdge => SamplerAddressMode::ClampToEdge,
            WrappingMode::MirroredRepeat => SamplerAddressMode::MirroredRepeat,
            WrappingMode::Repeat => SamplerAddressMode::Repeat,
        },
        address_mode_v: match sampler.wrap_t() {
            WrappingMode::ClampToEdge => SamplerAddressMode::ClampToEdge,
            WrappingMode::MirroredRepeat => SamplerAddressMode::MirroredRepeat,
            WrappingMode::Repeat => SamplerAddressMode::Repeat,
        },
        address_mode_w: SamplerAddressMode::ClampToBorder,
        mip_lod_bias: 0.0.into(),
        max_anisotropy: None,
        compare_op: None,
        min_lod: 0.0.into(),
        max_lod: 100.0.into(),
        border_color: BorderColor::FloatOpaqueWhite,
        unnormalized_coordinates: false,
    })
}
