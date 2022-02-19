use std::hash::{Hash, Hasher};

use goods::AssetField;
use ordered_float::OrderedFloat;

use super::texture::Texture;

#[derive(Clone, Debug, AssetField)]
pub struct Material {
    #[asset(container)]
    pub metalness_roughness: Option<Texture>,
    #[asset(container)]
    pub albedo: Option<Texture>,
    #[asset(container)]
    pub emissive: Option<Texture>,
    #[asset(container)]
    pub transmission: Option<Texture>,
    #[asset(container)]
    pub normal: Option<Texture>,
    pub albedo_factor: [f32; 4],
    pub metalness_factor: f32,
    pub roughness_factor: f32,
    pub emissive_factor: [f32; 3],
    pub transmission_factor: f32,
    pub normal_factor: f32, /* normal_in_tangent_space =
                             * vec3(sampled_normal.xy
                             * * normal_factor,
                             * sampled_normal.z) */
}

impl PartialEq for Material {
    fn eq(&self, other: &Self) -> bool {
        if self.albedo != other.albedo {
            return false;
        }
        if self.metalness_roughness != other.metalness_roughness {
            return false;
        }
        if self.emissive != other.emissive {
            return false;
        }
        if self.transmission != other.transmission {
            return false;
        }
        if self.normal != other.normal {
            return false;
        }

        if self.albedo_factor.map(OrderedFloat) != other.albedo_factor.map(OrderedFloat) {
            return false;
        }
        if OrderedFloat(self.metalness_factor) != OrderedFloat(other.metalness_factor) {
            return false;
        }
        if OrderedFloat(self.roughness_factor) != OrderedFloat(other.roughness_factor) {
            return false;
        }
        if self.emissive_factor.map(OrderedFloat) != other.emissive_factor.map(OrderedFloat) {
            return false;
        }
        if OrderedFloat(self.transmission_factor) != OrderedFloat(other.transmission_factor) {
            return false;
        }
        if OrderedFloat(self.normal_factor) != OrderedFloat(other.normal_factor) {
            return false;
        }
        true
    }
}

impl Eq for Material {}

impl Hash for Material {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.albedo.hash(state);
        self.metalness_roughness.hash(state);
        self.emissive.hash(state);
        self.transmission.hash(state);
        self.normal.hash(state);
        self.albedo_factor.map(OrderedFloat).hash(state);
        OrderedFloat(self.metalness_factor).hash(state);
        OrderedFloat(self.roughness_factor).hash(state);
        self.emissive_factor.map(OrderedFloat).hash(state);
        OrderedFloat(self.transmission_factor).hash(state);
        OrderedFloat(self.normal_factor).hash(state);
    }
}

impl Default for Material {
    fn default() -> Self {
        Material::new()
    }
}

impl Material {
    pub const fn new() -> Self {
        Material {
            albedo: None,
            metalness_roughness: None,
            emissive: None,
            transmission: None,
            normal: None,
            albedo_factor: defaults::albedo_factor(),
            metalness_factor: defaults::metalness_factor(),
            roughness_factor: defaults::roughness_factor(),
            emissive_factor: defaults::emissive_factor(),
            transmission_factor: defaults::transmission_factor(),
            normal_factor: defaults::normal_factor(),
        }
    }

    pub const fn color(rgba: [f32; 4]) -> Self {
        let mut material = Material::new();
        material.albedo_factor = rgba;
        material
    }

    pub const fn with_metalness(mut self, factor: f32) -> Self {
        self.metalness_factor = factor;
        self
    }

    pub const fn with_roughness(mut self, factor: f32) -> Self {
        self.roughness_factor = factor;
        self
    }
}

mod defaults {
    pub const fn albedo_factor() -> [f32; 4] {
        [1.0; 4]
    }

    pub const fn metalness_factor() -> f32 {
        0.0
    }

    pub const fn roughness_factor() -> f32 {
        1.0
    }

    pub const fn emissive_factor() -> [f32; 3] {
        [0.0; 3]
    }

    pub const fn transmission_factor() -> f32 {
        0.0
    }

    pub const fn normal_factor() -> f32 {
        1.0
    }
}
