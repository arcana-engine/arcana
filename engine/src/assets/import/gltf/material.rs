use crate::assets::{material::MaterialInfo, texture::TextureInfo};

pub fn load_material(material: gltf::Material, textures: &[TextureInfo]) -> MaterialInfo {
    let pbr = material.pbr_metallic_roughness();

    MaterialInfo {
        albedo: match pbr.base_color_texture() {
            Some(info) => Some(textures[info.texture().index()]),
            None => None,
        },
        albedo_factor: {
            let [r, g, b, a] = pbr.base_color_factor();
            [r, g, b, a]
        },
        metalness_roughness: match pbr.metallic_roughness_texture() {
            Some(info) => Some(textures[info.texture().index()]),
            None => None,
        },
        metalness_factor: pbr.metallic_factor(),
        roughness_factor: pbr.roughness_factor(),

        emissive: match material.emissive_texture() {
            Some(info) => Some(textures[info.texture().index()]),
            None => None,
        },
        emissive_factor: {
            let [r, g, b] = material.emissive_factor();
            [r, g, b]
        },

        transmission: None,
        transmission_factor: 0.0,

        normal: match material.normal_texture() {
            Some(info) => Some(textures[info.texture().index()]),
            None => None,
        },
        normal_factor: material
            .normal_texture()
            .map(|info| info.scale())
            .unwrap_or(0.0),
    }
}
