use std::path::Path;

use goods_treasury_import::Registry;

use crate::{
    material::{Material, Texture},
    sampler::Sampler,
};

fn load_texture(
    texture: gltf::Texture,
    samplers: &[Option<Sampler>],
    path: &Path,
    registry: &mut dyn Registry,
) -> eyre::Result<Texture> {
    match texture.source().source() {
        gltf::image::Source::View { .. } => unimplemented!(),
        gltf::image::Source::Uri { uri, .. } => {
            let buf;
            let source_path: &Path = match path.parent() {
                None => path.as_ref(),
                Some(parent) => {
                    buf = parent.join(uri);
                    &buf
                }
            };

            let image = registry.store(source_path, "image", "rgba.png", &["texture", "gltf"])?;

            let sampler = texture.sampler().index().and_then(|idx| samplers[idx]);
            let texture = match sampler {
                None => Texture::Image(image),
                Some(sampler) => Texture::ImageSampler { image, sampler },
            };

            Ok(texture)
        }
    }
}

pub fn load_material(
    material: gltf::Material,
    samplers: &[Option<Sampler>],
    path: &Path,
    registry: &mut dyn Registry,
) -> eyre::Result<Material> {
    let pbr = material.pbr_metallic_roughness();

    Ok(Material {
        albedo_coverage: match pbr.base_color_texture() {
            Some(info) => Some(load_texture(info.texture(), samplers, path, registry)?),
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
            Some(info) => Some(load_texture(info.texture(), samplers, path, registry)?),
            None => None,
        },
        metalness_factor: pbr.metallic_factor().into(),
        roughness_factor: pbr.roughness_factor().into(),

        emissive: match material.emissive_texture() {
            Some(info) => Some(load_texture(info.texture(), samplers, path, registry)?),
            None => None,
        },
        emissive_factor: {
            let [r, g, b] = material.emissive_factor();
            [r.into(), g.into(), b.into()]
        },

        transmission: None,
        transmission_factor: 0.0.into(),

        normal: match material.normal_texture() {
            Some(info) => Some(load_texture(info.texture(), samplers, path, registry)?),
            None => None,
        },
        normal_factor: material
            .normal_texture()
            .map(|info| info.scale())
            .unwrap_or(0.0)
            .into(),
    })
}
