use {super::texture::Texture, ordered_float::OrderedFloat};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Material {
    pub albedo_coverage: Option<Texture>,
    pub metalness_roughness: Option<Texture>,
    pub emissive: Option<Texture>,
    pub transmission: Option<Texture>,
    pub normal: Option<Texture>,
    pub albedo_factor: [OrderedFloat<f32>; 3],
    pub coverage_factor: OrderedFloat<f32>,
    pub metalness_factor: OrderedFloat<f32>,
    pub roughness_factor: OrderedFloat<f32>,
    pub emissive_factor: [OrderedFloat<f32>; 3],
    pub transmission_factor: OrderedFloat<f32>,
    pub normal_factor: OrderedFloat<f32>, /* normal_in_tangent_space =
                                           * vec3(sampled_normal.xy
                                           * * normal_factor,
                                           * sampled_normal.z) */
}

impl Default for Material {
    fn default() -> Self {
        Material::new()
    }
}

impl Material {
    pub const fn new() -> Material {
        Material {
            albedo_coverage: None,
            metalness_roughness: None,
            emissive: None,
            transmission: None,
            normal: None,
            albedo_factor: [OrderedFloat(1.0); 3],
            coverage_factor: OrderedFloat(1.0),
            metalness_factor: OrderedFloat(0.0),
            roughness_factor: OrderedFloat(1.0),
            emissive_factor: [OrderedFloat(0.0); 3],
            transmission_factor: OrderedFloat(0.0),
            normal_factor: OrderedFloat(1.0),
        }
    }

    pub fn color(rgb: [f32; 3]) -> Self {
        let [r, g, b] = rgb;
        Material {
            albedo_factor: [r.into(), g.into(), b.into()],
            ..Material::new()
        }
    }

    pub fn with_metalness(mut self, factor: f32) -> Self {
        self.metalness_factor = factor.into();
        self
    }

    pub fn with_roughness(mut self, factor: f32) -> Self {
        self.roughness_factor = factor.into();
        self
    }
}
