// mod animation;
mod collider;
mod image;
mod material;
mod mesh;
mod primitive;
mod sampler;
mod skin;

use std::{collections::HashMap, fs::File, io::Write, path::Path};

use gltf::{
    accessor::{DataType, Dimensions},
    Gltf,
};

use sierra::SamplerInfo;
use treasury_import::{Dependencies, Dependency, ImportError, Importer, Sources};

use crate::assets::{
    import::gltf::{material::load_material, mesh::load_mesh, skin::load_skin},
    model::ModelFileHeader,
    texture::TextureInfo,
};

use self::sampler::load_sampler;

/// Imports single object with one or more mesh primitives, colliders and animations (not yet).
pub struct GltfModelImporter;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Accessor has unexpected dimensions `{unexpected:?}`. Expected `{expected:?}`")]
    UnexpectedDimensions {
        unexpected: Dimensions,
        expected: &'static [Dimensions],
    },

    #[error("Accessor has unexpected data type `{unexpected:?}`. Expected `{expected:?}`")]
    UnexpectedDataType {
        unexpected: DataType,
        expected: &'static [DataType],
    },

    #[error("Sparse accessors are not supported")]
    SparseAccessorUnsupported,

    #[error("Accessor does not fit the view")]
    AccessorOutOfBound,

    #[error("View does not fit the source")]
    ViewOutOfBound,

    #[error("Unsupported mesh without position attribute")]
    MissingPositionAttribute,

    #[error("Unsupported mesh topology")]
    UnsupportedTopology { unsupported: gltf::mesh::Mode },

    #[error("Invalid convex shape provided")]
    InvalidConvexShape,

    #[error("View stride is less than accessor size")]
    InvalidViewStride,

    #[error("Integer overflow")]
    IntegerOverflow,

    #[error("Buffer refers to missing bin part of glTF")]
    MissingBin,
}

impl Importer for GltfModelImporter {
    fn name(&self) -> &str {
        "glTF model"
    }

    fn formats(&self) -> &[&str] {
        &["gltf"]
    }

    fn extensions(&self) -> &[&str] {
        &["json", "bin", "gltf"]
    }

    fn target(&self) -> &str {
        "arcana.model"
    }

    fn import(
        &self,
        source: &Path,
        output: &Path,
        sources: &mut (impl Sources + ?Sized),
        dependencies: &mut (impl Dependencies + ?Sized),
    ) -> Result<(), ImportError> {
        let gltf = gltf::Gltf::open(source).map_err(|err| ImportError::Other {
            reason: format!("Failed to open glTF file '{}'. {:#}", source.display(), err),
        })?;

        let mut missing_sources = Vec::new();
        let mut missing_dependencies = Vec::new();

        let mut buffers = HashMap::new();

        for buffer in gltf.buffers() {
            match buffer.source() {
                gltf::buffer::Source::Bin => {}
                gltf::buffer::Source::Uri(uri) => {
                    let source_path = sources
                        .get_or_append(uri, &mut missing_sources)
                        .map_err(|reason| ImportError::Other { reason })?;

                    if missing_sources.is_empty() {
                        let source_path = source_path.as_deref().unwrap();
                        let data =
                            std::fs::read(source_path).map_err(|err| ImportError::Other {
                                reason: format!(
                                    "Failed to load source '{}' from file '{}'. {:#}",
                                    uri,
                                    source_path.display(),
                                    err
                                ),
                            })?;
                        buffers.insert(buffer.index(), data.into_boxed_slice());
                    }
                }
            }
        }

        if !missing_sources.is_empty() {
            return Err(ImportError::RequireSources {
                sources: missing_sources,
            });
        }

        let mut samplers = Vec::new();

        for sampler in gltf.samplers() {
            samplers.push(load_sampler(sampler));
        }

        let mut textures = Vec::new();

        for texture in gltf.textures() {
            if missing_dependencies.is_empty() {
                if let Some(texture) =
                    load_texture(texture, &samplers, dependencies, &mut missing_dependencies)
                        .map_err(|reason| ImportError::Other { reason })?
                {
                    textures.push(texture);
                }
            } else {
                get_missing_texture_dependencies(texture, dependencies, &mut missing_dependencies)
                    .map_err(|reason| ImportError::Other { reason })?;
            }
        }

        if !missing_dependencies.is_empty() {
            return Err(ImportError::RequireDependencies {
                dependencies: missing_dependencies,
            });
        }

        let scene = gltf.default_scene().ok_or_else(|| ImportError::Other {
            reason: format!("Unexpected glTF structure"),
        })?;
        let root = scene.nodes().next().ok_or_else(|| ImportError::Other {
            reason: format!("Unexpected glTF structure"),
        })?;

        let materials: Vec<_> = gltf
            .materials()
            .map(|material| load_material(material, &textures))
            .collect();

        let mesh_node = match root.mesh() {
            None => root
                .children()
                .find(|node| node.mesh().is_some())
                .ok_or_else(|| ImportError::Other {
                    reason: format!("Unexpected glTF structure"),
                })?,
            Some(_) => root,
        };

        let mut mesh_data = vec![0u8; 0];

        let mesh = load_mesh(mesh_node.mesh().unwrap(), &gltf, &buffers, &mut mesh_data).map_err(
            |err| ImportError::Other {
                reason: format!(
                    "Failed to load mesh {} from glTF '{}'. {:#}",
                    mesh_node.index(),
                    source.display(),
                    err
                ),
            },
        )?;

        let mut model = match mesh_node.skin() {
            None => ModelFileHeader {
                magic: ModelFileHeader::MAGIC,
                primitives: mesh.primitives,
                colliders: mesh.colliders,
                skin: None,
                materials,
            },
            Some(skin) => {
                let skin =
                    load_skin(skin.clone(), &gltf, &buffers).map_err(|err| ImportError::Other {
                        reason: format!(
                            "Failed to load skin {} from glTF '{}'. {:#}",
                            skin.index(),
                            source.display(),
                            err
                        ),
                    })?;
                ModelFileHeader {
                    magic: ModelFileHeader::MAGIC,
                    primitives: mesh.primitives,
                    colliders: mesh.colliders,
                    skin: Some(skin),
                    materials,
                }
            }
        };

        let mut output_file = File::create(output).map_err(|err| ImportError::Other {
            reason: format!("Failed open output file '{}'. {:#}", output.display(), err),
        })?;

        let header_size = bincode::serialized_size(&model).map_err(|err| ImportError::Other {
            reason: format!("Failed to determine size of the object header. {:#}", err),
        })?;

        assert_eq!(header_size as usize as u64, header_size);
        let header_size = header_size as usize;

        for primitive in &mut model.primitives {
            if let Some(indices) = &mut primitive.indices {
                indices.offset += header_size;
            }

            for binding in &mut primitive.bindings {
                binding.offset += header_size;
            }
        }

        bincode::serialize_into(&mut output_file, &model).map_err(|err| ImportError::Other {
            reason: format!(
                "Failed to serialize object header to '{}'. {:#}",
                output.display(),
                err
            ),
        })?;

        output_file
            .write_all(&mesh_data)
            .map_err(|err| ImportError::Other {
                reason: format!(
                    "Failed write mesh data to '{}'. {:#}",
                    output.display(),
                    err
                ),
            })?;

        Ok(())
    }
}

fn read_accessor<'a>(
    accessor: gltf::Accessor<'_>,
    gltf: &'a Gltf,
    buffers: &'a HashMap<usize, Box<[u8]>>,
) -> Result<(&'a [u8], usize), Error> {
    let view = accessor.view().ok_or(Error::SparseAccessorUnsupported)?;

    let stride = view.stride().unwrap_or(accessor.size());
    if stride < accessor.size() {
        tracing::error!(
            "Accessor '{}' with size '{}' is bound to view '{}' with insufficient stride '{}'",
            accessor.index(),
            accessor.size(),
            view.index(),
            stride,
        );
        return Err(Error::InvalidViewStride);
    }

    // Total byte count for accessor.
    let accessor_size = if accessor.count() == 0 {
        0
    } else {
        (accessor.count() - 1) * stride + accessor.size()
    };

    if view.length() < accessor_size + accessor.offset() {
        tracing::error!(
            "Accessor '{}' is out of buffer view bounds",
            accessor.index(),
        );
        return Err(Error::AccessorOutOfBound);
    }

    let bytes = match view.buffer().source() {
        gltf::buffer::Source::Bin => gltf.blob.as_deref().ok_or(Error::MissingBin)?,
        gltf::buffer::Source::Uri(_) => &buffers[&view.buffer().index()],
    };

    if bytes.len() < view.offset() + view.length() {
        tracing::error!("View '{}' is out of buffer bounds", view.index(),);
        return Err(Error::ViewOutOfBound);
    }

    let bytes = &bytes[view.offset() + accessor.offset()..][..accessor_size];
    Ok((bytes, stride))
}

fn align_vec(bytes: &mut Vec<u8>, align_mask: usize) {
    let new_size = (bytes.len() + align_mask) & !align_mask;
    bytes.resize(new_size, 0xfe);
}

fn load_texture(
    texture: gltf::Texture,
    samplers: &[Option<SamplerInfo>],
    dependencies: &mut (impl Dependencies + ?Sized),
    missing: &mut Vec<Dependency>,
) -> Result<Option<TextureInfo>, String> {
    match texture.source().source() {
        gltf::image::Source::View { .. } => unimplemented!(),
        gltf::image::Source::Uri { uri, .. } => {
            let image = dependencies.get_or_append(uri, "qoi", missing)?;

            match image {
                None => Ok(None),
                Some(image) => {
                    let image = goods::AssetId(image.value());
                    let sampler = texture.sampler().index().and_then(|idx| samplers[idx]);
                    let texture = match sampler {
                        None => TextureInfo::image(image),
                        Some(sampler) => TextureInfo { image, sampler },
                    };
                    Ok(Some(texture))
                }
            }
        }
    }
}

fn get_missing_texture_dependencies(
    texture: gltf::Texture,
    dependencies: &mut (impl Dependencies + ?Sized),
    missing: &mut Vec<Dependency>,
) -> Result<(), String> {
    match texture.source().source() {
        gltf::image::Source::View { .. } => unimplemented!(),
        gltf::image::Source::Uri { uri, .. } => {
            dependencies.get_or_append(uri, "qoi", missing)?;
            Ok(())
        }
    }
}
