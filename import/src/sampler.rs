use crate::is_default;

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum CompareOp {
    /// Never passes.
    Never,

    /// Passes if fragment's depth is less than stored.
    Less,

    /// Passes if fragment's depth is equal to stored.
    Equal,

    /// Passes if fragment's depth is less than or equal to stored.
    LessOrEqual,

    /// Passes if fragment's depth is greater than stored.
    Greater,

    /// Passes if fragment's depth is not equal to stored.
    NotEqual,

    /// Passes if fragment's depth is greater than or equal to stored.
    GreaterOrEqual,

    /// Always passes.
    Always,
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Filter {
    Nearest,
    Linear,
    // Cubic,
}

impl Default for Filter {
    fn default() -> Self {
        Filter::Nearest
    }
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum MipmapMode {
    Nearest,
    Linear,
}

impl Default for MipmapMode {
    fn default() -> Self {
        MipmapMode::Nearest
    }
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum SamplerAddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
    MirrorClampToEdge,
}

impl Default for SamplerAddressMode {
    fn default() -> Self {
        SamplerAddressMode::ClampToEdge
    }
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum BorderColor {
    FloatTransparentBlack,
    IntTransparentBlack,
    FloatOpaqueBlack,
    IntOpaqueBlack,
    FloatOpaqueWhite,
    IntOpaqueWhite,
}

impl Default for BorderColor {
    fn default() -> Self {
        BorderColor::FloatTransparentBlack
    }
}

#[derive(Clone, Copy, PartialEq, serde::Serialize)]
pub struct Sampler {
    #[serde(skip_serializing_if = "is_default")]
    pub mag_filter: Filter,
    #[serde(skip_serializing_if = "is_default")]
    pub min_filter: Filter,
    #[serde(skip_serializing_if = "is_default")]
    pub mipmap_mode: MipmapMode,
    #[serde(skip_serializing_if = "is_default")]
    pub address_mode_u: SamplerAddressMode,
    #[serde(skip_serializing_if = "is_default")]
    pub address_mode_v: SamplerAddressMode,
    #[serde(skip_serializing_if = "is_default")]
    pub address_mode_w: SamplerAddressMode,
    #[serde(skip_serializing_if = "is_default")]
    pub mip_lod_bias: f32,
    #[serde(skip_serializing_if = "is_default")]
    pub max_anisotropy: Option<f32>,
    #[serde(skip_serializing_if = "is_default")]
    pub compare_op: Option<CompareOp>,
    #[serde(skip_serializing_if = "is_default")]
    pub min_lod: f32,
    #[serde(skip_serializing_if = "is_default")]
    pub max_lod: f32,
    #[serde(skip_serializing_if = "is_default")]
    pub border_color: BorderColor,
    #[serde(skip_serializing_if = "is_default")]
    pub unnormalized_coordinates: bool,
}

impl Default for Sampler {
    fn default() -> Self {
        Sampler {
            mag_filter: Filter::Nearest,
            min_filter: Filter::Nearest,
            mipmap_mode: MipmapMode::Nearest,
            address_mode_u: SamplerAddressMode::ClampToEdge,
            address_mode_v: SamplerAddressMode::ClampToEdge,
            address_mode_w: SamplerAddressMode::ClampToEdge,
            mip_lod_bias: 0.0,
            max_anisotropy: None,
            compare_op: None,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: BorderColor::FloatTransparentBlack,
            unnormalized_coordinates: false,
        }
    }
}
