use crate::{sampler::Sampler, AssetId};

#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum Texture {
    ImageSampler { image: AssetId, sampler: Sampler },
    Image(AssetId),
}

#[derive(serde::Serialize)]
pub struct Material {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub albedo_coverage: Option<Texture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metalness_roughness: Option<Texture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emissive: Option<Texture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transmission: Option<Texture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normal: Option<Texture>,
    pub albedo_factor: [f32; 3],
    pub coverage_factor: f32,
    pub metalness_factor: f32,
    pub roughness_factor: f32,
    pub emissive_factor: [f32; 3],
    pub transmission_factor: f32,
    pub normal_factor: f32,
}
