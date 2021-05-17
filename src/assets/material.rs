use {
    super::{image::ImageAsset, AssetResult, Loader},
    crate::graphics::{Graphics, Material, Texture},
    ordered_float::OrderedFloat,
    sierra::SamplerInfo,
    url::Url,
};

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct MaterialInfo {
    #[serde(default)]
    pub albedo_coverage: Option<TextureInfo>,

    #[serde(default)]
    pub metalness_roughness: Option<TextureInfo>,

    #[serde(default)]
    pub emissive: Option<TextureInfo>,

    #[serde(default)]
    pub transmission: Option<TextureInfo>,

    #[serde(default)]
    pub normal: Option<TextureInfo>,

    #[serde(default = "defaults::albedo_factor")]
    pub albedo_factor: [OrderedFloat<f32>; 3],

    #[serde(default = "defaults::coverage_factor")]
    pub coverage_factor: OrderedFloat<f32>,

    #[serde(default = "defaults::metalness_factor")]
    pub metalness_factor: OrderedFloat<f32>,

    #[serde(default = "defaults::roughness_factor")]
    pub roughness_factor: OrderedFloat<f32>,

    #[serde(default = "defaults::emissive_factor")]
    pub emissive_factor: [OrderedFloat<f32>; 3],

    #[serde(default = "defaults::transmission_factor")]
    pub transmission_factor: OrderedFloat<f32>,

    #[serde(default = "defaults::normal_factor")]
    pub normal_factor: OrderedFloat<f32>,
}

mod defaults {
    use ordered_float::OrderedFloat;

    pub const fn albedo_factor() -> [OrderedFloat<f32>; 3] {
        [OrderedFloat(1.0); 3]
    }

    pub const fn coverage_factor() -> OrderedFloat<f32> {
        OrderedFloat(1.0)
    }

    pub const fn metalness_factor() -> OrderedFloat<f32> {
        OrderedFloat(0.0)
    }

    pub const fn roughness_factor() -> OrderedFloat<f32> {
        OrderedFloat(1.0)
    }

    pub const fn emissive_factor() -> [OrderedFloat<f32>; 3] {
        [OrderedFloat(0.0); 3]
    }

    pub const fn transmission_factor() -> OrderedFloat<f32> {
        OrderedFloat(0.0)
    }

    pub const fn normal_factor() -> OrderedFloat<f32> {
        OrderedFloat(1.0)
    }
}

impl MaterialInfo {
    pub async fn load(self, parent: Option<&Url>, loader: Loader) -> MaterialDecoded {
        MaterialDecoded {
            albedo_coverage: if let Some(info) = self.albedo_coverage {
                Some(info.load(parent, &loader).await)
            } else {
                None
            },
            metalness_roughness: if let Some(info) = self.metalness_roughness {
                Some(info.load(parent, &loader).await)
            } else {
                None
            },
            emissive: if let Some(info) = self.emissive {
                Some(info.load(parent, &loader).await)
            } else {
                None
            },
            transmission: if let Some(info) = self.transmission {
                Some(info.load(parent, &loader).await)
            } else {
                None
            },
            normal: if let Some(info) = self.normal {
                Some(info.load(parent, &loader).await)
            } else {
                None
            },
            albedo_factor: self.albedo_factor,
            coverage_factor: self.coverage_factor,
            metalness_factor: self.metalness_factor,
            roughness_factor: self.roughness_factor,
            emissive_factor: self.emissive_factor,
            transmission_factor: self.transmission_factor,
            normal_factor: self.normal_factor,
        }
    }
}
