use std::{borrow::BorrowMut, sync::Arc};

use futures::future::BoxFuture;
use goods::{Asset, AssetBuild, AssetField, AssetFieldBuild, Container, Loader};

#[cfg(feature = "graphics")]
use sierra::{OutOfMemory, PrimitiveTopology};

#[cfg(feature = "graphics")]
use skelly::Skelly;

#[cfg(feature = "graphics")]
use crate::graphics::{
    BindingFileHeader, Graphics, IndicesFileHeader, Material, MaterialBuildError,
    MaterialDecodeError, MaterialDecoded, MaterialInfo, Mesh,
};

#[cfg(feature = "graphics")]
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

#[cfg(feature = "graphics")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Skin {
    pub inverse_binding_matrices: Option<Vec<na::Matrix4<f32>>>,
    pub skelly: Skelly<f32, String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ModelFileHeader {
    pub magic: u32,
    pub colliders: Vec<Collider>,
    #[cfg(feature = "graphics")]
    pub primitives: Vec<PrimitiveInfo>,
    #[cfg(feature = "graphics")]
    pub skin: Option<Skin>,
    #[cfg(feature = "graphics")]
    pub materials: Vec<MaterialInfo>,
}

pub struct ModelFileDecoded {
    colliders: Vec<Collider>,
    #[cfg(feature = "graphics")]
    primitives: Vec<PrimitiveInfo>,
    #[cfg(feature = "graphics")]
    skin: Option<Skin>,
    #[cfg(feature = "graphics")]
    materials: Vec<MaterialDecoded>,
    #[cfg(feature = "graphics")]
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

    #[error("Failed to deserialize model file header")]
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
                    #[cfg(feature = "graphics")]
                    let mut materials = Vec::new();

                    #[cfg(feature = "graphics")]
                    {
                        for material in header.materials {
                            let decoded = Material::decode(material, &loader).await?;
                            materials.push(decoded);
                        }
                    }

                    Ok(ModelFileDecoded {
                        colliders: header.colliders,

                        #[cfg(feature = "graphics")]
                        primitives: header.primitives,

                        #[cfg(feature = "graphics")]
                        skin: header.skin,

                        #[cfg(feature = "graphics")]
                        materials,

                        #[cfg(feature = "graphics")]
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
        #[cfg(feature = "graphics")]
        let mut primitives = Vec::new();

        #[cfg(feature = "graphics")]
        {
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
        }

        #[cfg(feature = "graphics")]
        let mut materials = Vec::new();

        #[cfg(feature = "graphics")]
        {
            for material in decoded.materials {
                let material =
                    <Material as AssetFieldBuild<Container, B>>::build(material, builder)?;
                materials.push(material);
            }
        }

        Ok(Model {
            colliders: decoded.colliders.into(),
            #[cfg(feature = "graphics")]
            primitives: primitives.into(),
            #[cfg(feature = "graphics")]
            skin: decoded.skin,
            #[cfg(feature = "graphics")]
            materials: materials.into(),
        })
    }
}
