mod animation;
mod collider;
mod image;
mod material;
mod mesh;
mod primitive;
mod sampler;
mod skin;

use std::{collections::HashMap, fs::File, path::Path};

use gltf::{
    accessor::{DataType, Dimensions},
    Gltf,
};
use goods_treasury_import::{Importer, Registry};

use crate::{
    gltf::{material::load_material, sampler::load_sampler},
    material::Material,
};

use self::{
    collider::Collider,
    mesh::load_mesh,
    primitive::Primitive,
    skin::{load_skin, Skin},
};

/// Imports single object with one or more mesh primitives, colliders and animations.
pub struct GltfObjectImporter;

#[derive(serde::Serialize)]
struct Object {
    primitives: Vec<Primitive>,
    colliders: Vec<Collider>,
    skin: Option<Skin>,
    materials: Vec<Material>,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(
        "Importer expects single scene with single root node with meshes, colliders and skeleton children"
    )]
    UnexpectedStructure,

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

    #[error("Source does not exist")]
    MissingSource,

    #[error("Unsupported mesh without position attribute")]
    MissingPositionAttribute,

    #[error("Unsupported mesh topology")]
    UnsupportedTopology { unsupported: gltf::mesh::Mode },

    #[error("Invalid convex shape provided")]
    InvalidConvexShape,

    #[error("View stride is less than accessor size")]
    InvalidViewStride,

    #[error("Integer overflow")]
    Overflow,

    #[error("IO error")]
    Io {
        #[from]
        source: std::io::Error,
    },
}

impl Importer for GltfObjectImporter {
    fn name(&self) -> &str {
        "arcana.gltf.object"
    }

    fn source(&self) -> &str {
        "gltf"
    }

    fn native(&self) -> &str {
        "arcana.object"
    }

    fn import(
        &self,
        source_path: &Path,
        native_path: &Path,
        registry: &mut dyn Registry,
    ) -> eyre::Result<()> {
        let gltf = gltf::Gltf::open(source_path)?;

        let sources = gltf
            .buffers()
            .filter_map(|b| match b.source() {
                gltf::buffer::Source::Bin => None,
                gltf::buffer::Source::Uri(uri) if uri.starts_with("data:") => {
                    match uri.strip_prefix("data:application/octet-stream;base64,") {
                        None => Some(Err(eyre::eyre!(
                            "Only base64 octet-stream data URLs are supported"
                        ))),
                        Some(data) => {
                            let data = base64::decode(data).expect("Valid base64 expected");
                            Some(Ok((b.index(), data.into_boxed_slice())))
                        }
                    }
                }
                gltf::buffer::Source::Uri(uri) => {
                    let buf;
                    let source_path: &Path = match source_path.parent() {
                        None => source_path.as_ref(),
                        Some(parent) => {
                            buf = parent.join(uri);
                            &buf
                        }
                    };
                    match std::fs::read(source_path).map(Into::into) {
                        Ok(data) => Some(Ok((b.index(), data))),
                        Err(err) => Some(Err(err.into())),
                    }
                }
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        let scene = gltf.default_scene().ok_or(Error::UnexpectedStructure)?;
        let root = scene.nodes().next().ok_or(Error::UnexpectedStructure)?;

        let samplers: Vec<_> = gltf
            .samplers()
            .map(|sampler| load_sampler(sampler))
            .collect();
        let materials: Vec<_> = gltf
            .materials()
            .map(|material| load_material(material, &samplers, source_path, registry))
            .collect::<Result<_, _>>()?;

        let mesh_node = match root.mesh() {
            None => root
                .children()
                .find(|node| node.mesh().is_some())
                .ok_or(Error::UnexpectedStructure)?,
            Some(_) => root,
        };

        let mesh = load_mesh(
            mesh_node.mesh().unwrap(),
            &gltf,
            &sources,
            source_path,
            registry,
        )?;

        let object = match mesh_node.skin() {
            None => Object {
                primitives: mesh.primitives,
                colliders: mesh.colliders,
                skin: None,
                materials,
            },
            Some(skin) => {
                let skin = load_skin(skin, &gltf, &sources)?;
                Object {
                    primitives: mesh.primitives,
                    colliders: mesh.colliders,
                    skin: Some(skin),
                    materials,
                }
            }
        };

        let mut native_file = File::create(native_path)?;
        serde_json::to_writer(&mut native_file, &object)?;
        Ok(())
    }
}

fn read_accessor<'a>(
    accessor: gltf::Accessor<'_>,
    gltf: &'a Gltf,
    sources: &'a HashMap<usize, Box<[u8]>>,
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
        gltf::buffer::Source::Bin => gltf.blob.as_deref().ok_or_else(|| {
            tracing::error!("View '{}' has non-existent bin", view.index());
            Error::MissingSource
        })?,
        gltf::buffer::Source::Uri(uri) => sources.get(&view.buffer().index()).ok_or_else(|| {
            tracing::error!("View '{}' has non-existent source {}", view.index(), uri);
            Error::MissingSource
        })?,
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
