use {
    super::{GltfBuildContext, GltfLoadingError},
    std::collections::hash_map::Entry,
};

impl GltfBuildContext<'_> {
    pub fn get_material(&mut self, material: gltf::Material) -> Result<Material, GltfLoadingError> {
        let index = material.index();
        match self.materials.entry(material.index()) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(_) => {
                let material = self.create_material(material)?;
                Ok(self.materials.entry(index).or_insert(material).clone())
            }
        }
    }

    fn get_texture(
        &mut self,
        texture: gltf::Texture,
        srgb: bool,
    ) -> Result<Texture, GltfLoadingError> {
        let image = self.get_image(texture.source(), srgb)?;
        let sampler = self.get_sampler(texture.sampler())?;
        Ok(Texture { image, sampler })
    }

    fn create_material(&mut self, material: gltf::Material) -> Result<Material, GltfLoadingError> {
        let pbr = material.pbr_metallic_roughness();

        Ok(Material {
            albedo_coverage: match pbr.base_color_texture() {
                Some(info) => Some(self.get_texture(info.texture(), true)?),
                None => None,
            },
            albedo_factor: {
                let [r, g, b, _] = pbr.base_color_factor();
                [r.into(), g.into(), b.into()]
            },
            coverage_factor: {
                let [_, _, _, a] = pbr.base_color_factor();
                a.into()
            },

            metalness_roughness: match pbr.metallic_roughness_texture() {
                Some(info) => Some(self.get_texture(info.texture(), false)?),
                None => None,
            },
            metalness_factor: pbr.metallic_factor().into(),
            roughness_factor: pbr.roughness_factor().into(),

            emissive: match material.emissive_texture() {
                Some(info) => Some(self.get_texture(info.texture(), true)?),
                None => None,
            },
            emissive_factor: {
                let [r, g, b] = material.emissive_factor();
                [r.into(), g.into(), b.into()]
            },

            transmission: None,
            transmission_factor: 0.0.into(),

            normal: match material.normal_texture() {
                Some(info) => Some(self.get_texture(info.texture(), false)?),
                None => None,
            },
            normal_factor: material
                .normal_texture()
                .map(|info| info.scale())
                .unwrap_or(0.0)
                .into(),
        })
    }
}
