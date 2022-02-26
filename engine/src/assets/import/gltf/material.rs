use crate::graphics::{MaterialInfo, TextureInfo};

pub fn load_material(material: gltf::Material, textures: &[TextureInfo]) -> MaterialInfo {
    let pbr = material.pbr_metallic_roughness();

    MaterialInfo {
        albedo: pbr
            .base_color_texture()
            .map(|info| textures[info.texture().index()]),
        albedo_factor: {
            let [r, g, b, a] = pbr.base_color_factor();
            [r, g, b, a]
        },
        metalness_roughness: pbr
            .metallic_roughness_texture()
            .map(|info| textures[info.texture().index()]),
        metalness_factor: pbr.metallic_factor(),
        roughness_factor: pbr.roughness_factor(),

        emissive: material
            .emissive_texture()
            .map(|info| textures[info.texture().index()]),
        emissive_factor: {
            let [r, g, b] = material.emissive_factor();
            [r, g, b]
        },

        transmission: None,
        transmission_factor: 0.0,

        normal: material
            .normal_texture()
            .map(|info| textures[info.texture().index()]),
        normal_factor: material
            .normal_texture()
            .map(|info| info.scale())
            .unwrap_or(0.0),
    }
}
