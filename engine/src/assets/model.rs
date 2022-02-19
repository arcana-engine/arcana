use std::{borrow::BorrowMut, sync::Arc};

use futures::future::BoxFuture;
use goods::{Asset, AssetBuild, AssetField, AssetFieldBuild, Container, Loader};

use sierra::{OutOfMemory, PrimitiveTopology};
use skelly::Skelly;

use crate::graphics::{Graphics, Mesh};

use super::{
    material::{Material, MaterialBuildError, MaterialDecodeError, MaterialDecoded, MaterialInfo},
    mesh::{BindingFileHeader, IndicesFileHeader},
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PrimitiveInfo {
    pub vertex_count: u32,
    pub bindings: Vec<BindingFileHeader>,
    pub indices: Option<IndicesFileHeader>,
    pub topology: PrimitiveTopology,
    pub material: Option<usize>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Collider {
    AABB {
        extent: na::Vector3<f32>,
    },
    Convex {
        points: Vec<na::Point3<f32>>,
    },
    TriMesh {
        vertices: Vec<na::Point3<f32>>,
        indices: Vec<[u32; 3]>,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Skin {
    pub inverse_binding_matrices: Option<Vec<na::Matrix4<f32>>>,
    pub skelly: Skelly<f32, String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ModelFileHeader {
    pub magic: u32,
    pub primitives: Vec<PrimitiveInfo>,
    pub colliders: Vec<Collider>,
    pub skin: Option<Skin>,
    pub materials: Vec<MaterialInfo>,
}

pub struct ModelFileDecoded {
    primitives: Vec<PrimitiveInfo>,
    colliders: Vec<Collider>,
    skin: Option<Skin>,
    materials: Vec<MaterialDecoded>,
    bytes: Box<[u8]>,
}

impl ModelFileHeader {
    pub const MAGIC: u32 = u32::from_le_bytes(*b"arcm");
}

#[derive(Clone, Debug)]
pub struct Model {
    pub primitives: Arc<[Primitive]>,
    pub colliders: Arc<[Collider]>,
    pub skin: Option<Skin>,
    pub materials: Arc<[Material]>,
}

#[derive(Clone, Debug)]
pub struct Primitive {
    pub mesh: Mesh,
    pub material: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
pub enum ModelDecodeError {
    #[error("Failed to verify magic number")]
    MagicError,

    #[error("Failed to deserialize magic file header")]
    HeaderError { source: bincode::Error },

    #[error("Failed to build material")]
    Material {
        #[from]
        source: MaterialDecodeError,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ModelBuildError {
    #[error("Failed to build mesh")]
    Mesh { source: OutOfMemory },

    #[error("Failed to build material")]
    Material {
        #[from]
        source: MaterialBuildError,
    },
}

impl Asset for Model {
    type Decoded = ModelFileDecoded;
    type DecodeError = ModelDecodeError;
    type BuildError = ModelBuildError;
    type Fut = BoxFuture<'static, Result<ModelFileDecoded, ModelDecodeError>>;

    fn name() -> &'static str {
        "arcana.model"
    }

    fn decode(bytes: Box<[u8]>, loader: &Loader) -> Self::Fut {
        match &*bytes {
            [a, b, c, d, ..] => {
                let magic = u32::from_le_bytes([*a, *b, *c, *d]);
                if magic != ModelFileHeader::MAGIC {
                    tracing::error!(
                        "Mesh blob contains wrong magic number '{:X}'. Expected '{:X}'",
                        magic,
                        ModelFileHeader::MAGIC
                    );
                    return Box::pin(async { Err(ModelDecodeError::MagicError) });
                }
            }
            _ => {
                tracing::error!("Mesh blob is too small");
                return Box::pin(async { Err(ModelDecodeError::MagicError) });
            }
        }

        match bincode::deserialize::<ModelFileHeader>(&*bytes) {
            Ok(header) => {
                debug_assert_eq!(header.magic, ModelFileHeader::MAGIC);

                let loader = loader.clone();

                Box::pin(async move {
                    let mut materials = Vec::new();
                    for material in header.materials {
                        let decoded = Material::decode(material, &loader).await?;
                        materials.push(decoded);
                    }

                    Ok(ModelFileDecoded {
                        primitives: header.primitives,
                        colliders: header.colliders,
                        skin: header.skin,
                        materials,
                        bytes,
                    })
                })
            }
            Err(err) => Box::pin(async { Err(ModelDecodeError::HeaderError { source: err }) }),
        }
    }
}

impl<B> AssetBuild<B> for Model
where
    B: BorrowMut<Graphics>,
{
    fn build(decoded: ModelFileDecoded, builder: &mut B) -> Result<Self, ModelBuildError> {
        let mut primitives = Vec::new();
        for primitive in decoded.primitives {
            let result = Mesh::build_from_file_data(
                primitive.vertex_count,
                &primitive.bindings,
                primitive.indices.as_ref(),
                primitive.topology,
                &decoded.bytes,
                builder.borrow_mut(),
            );

            match result {
                Ok(mesh) => {
                    primitives.push(Primitive {
                        mesh,
                        material: primitive.material,
                    });
                }

                Err(OutOfMemory) => {
                    return Err(ModelBuildError::Mesh {
                        source: OutOfMemory,
                    })
                }
            }
        }

        let mut materials = Vec::new();
        for material in decoded.materials {
            let material = <Material as AssetFieldBuild<Container, B>>::build(material, builder)?;
            materials.push(material);
        }

        Ok(Model {
            primitives: primitives.into(),
            colliders: decoded.colliders.into(),
            skin: decoded.skin,
            materials: materials.into(),
        })
    }
}
