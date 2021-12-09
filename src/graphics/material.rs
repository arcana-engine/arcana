use {super::texture::Texture, goods::AssetField, ordered_float::OrderedFloat};

#[derive(Clone, Debug, PartialEq, Eq, Hash, AssetField)]
pub struct Material {
    #[asset(container)]
    pub albedo_coverage: Option<Texture>,
    #[asset(container)]
    pub metalness_roughness: Option<Texture>,
    #[asset(container)]
    pub emissive: Option<Texture>,
    #[asset(container)]
    pub transmission: Option<Texture>,
    #[asset(container)]
    pub normal: Option<Texture>,
    pub albedo_factor: [OrderedFloat<f32>; 4],
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
            albedo_factor: defaults::albedo_factor(),
            coverage_factor: defaults::coverage_factor(),
            metalness_factor: defaults::metalness_factor(),
            roughness_factor: defaults::roughness_factor(),
            emissive_factor: defaults::emissive_factor(),
            transmission_factor: defaults::transmission_factor(),
            normal_factor: defaults::normal_factor(),
        }
    }

    pub const fn color(rgba: [f32; 4]) -> Self {
        let [r, g, b, a] = rgba;
        let mut material = Material::new();
        material.albedo_factor = [
            OrderedFloat(r),
            OrderedFloat(g),
            OrderedFloat(b),
            OrderedFloat(a),
        ];
        material
    }

    pub const fn with_metalness(mut self, factor: f32) -> Self {
        self.metalness_factor = OrderedFloat(factor);
        self
    }

    pub const fn with_roughness(mut self, factor: f32) -> Self {
        self.roughness_factor = OrderedFloat(factor);
        self
    }
}

mod defaults {
    use ordered_float::OrderedFloat;

    pub const fn albedo_factor() -> [OrderedFloat<f32>; 4] {
        [OrderedFloat(1.0); 4]
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
