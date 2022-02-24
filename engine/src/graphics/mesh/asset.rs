use std::{
    borrow::BorrowMut,
    convert::TryFrom,
    future::{ready, Ready},
};

use goods::{Asset, AssetBuild, Loader};
use sierra::{BufferInfo, BufferUsage, IndexType, OutOfMemory, PrimitiveTopology};

use crate::graphics::{
    Binding, Graphics, Indices, Joints, Mesh, Normal3, Position3, Tangent3, VertexLayout,
    VertexType, Weights, UV, V2, V3, V4,
};

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum MeshFileVertexLayout {
    Position3,
    Normal3,
    Tangent3,
    UV,
    ColorSrgba,
    PositionNormal3,
    PositionNormal3Color,
    PositionNormal3UV,
    PositionNormalTangent3,
    PositionNormalTangent3Color,
    PositionNormalTangent3UV,
    Joints,
    Weights,
    Skin,
}

impl MeshFileVertexLayout {
    /// Returns vertex layout for predefined mesh vertex layout.
    pub fn into_vertex_layout(&self) -> VertexLayout {
        match self {
            Self::Position3 => Position3::layout(),
            Self::Normal3 => Normal3::layout(),
            Self::Tangent3 => Tangent3::layout(),
            Self::UV => UV::layout(),
            Self::ColorSrgba => palette::Srgba::<u8>::layout(),
            Self::PositionNormal3 => V2::<Position3, Normal3>::layout(),
            Self::PositionNormal3UV => V3::<Position3, Normal3, UV>::layout(),
            Self::PositionNormal3Color => V3::<Position3, Normal3, palette::Srgba<u8>>::layout(),
            Self::PositionNormalTangent3 => V3::<Position3, Normal3, Tangent3>::layout(),
            Self::PositionNormalTangent3UV => V4::<Position3, Normal3, Tangent3, UV>::layout(),
            Self::PositionNormalTangent3Color => {
                V4::<Position3, Normal3, Tangent3, palette::Srgba<u8>>::layout()
            }
            Self::Joints => Joints::layout(),
            Self::Weights => Weights::layout(),
            Self::Skin => V2::<Joints, Weights>::layout(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BindingFileHeader {
    pub offset: usize,
    pub layout: MeshFileVertexLayout,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct IndicesFileHeader {
    pub offset: usize,
    pub count: u32,
    pub index_type: IndexType,
}

/// Header for internal mesh file format.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MeshFileHeader {
    pub magic: u32,
    pub vertex_count: u32,
    pub bindings: Vec<BindingFileHeader>,
    pub indices: Option<IndicesFileHeader>,
    pub topology: PrimitiveTopology,
}

impl MeshFileHeader {
    pub const MAGIC: u32 = u32::from_le_bytes(*b"msha");
}

impl Mesh {
    /// Build mesh from file data.
    pub fn build_from_file_data(
        vertex_count: u32,
        bindings: &[BindingFileHeader],
        indices: Option<&IndicesFileHeader>,
        topology: PrimitiveTopology,
        data: &[u8],
        graphics: &mut Graphics,
    ) -> Result<Self, OutOfMemory> {
        let bindings = bindings
            .iter()
            .map(|binding| -> Result<_, OutOfMemory> {
                let layout = binding.layout.into_vertex_layout();

                let size = u64::from(layout.stride) * u64::from(vertex_count);
                let size_usize = usize::try_from(size).map_err(|_| OutOfMemory)?;

                Ok(Binding {
                    buffer: graphics
                        .create_buffer_static(
                            BufferInfo {
                                align: 255,
                                size,
                                usage: BufferUsage::VERTEX,
                            },
                            &data[binding.offset..][..size_usize],
                        )?
                        .into(),
                    offset: 0,
                    layout,
                })
            })
            .collect::<Result<_, _>>()?;

        let mut count = vertex_count;

        let indices = indices
            .as_ref()
            .map(|indices| -> Result<_, OutOfMemory> {
                count = indices.count;

                let stride = match indices.index_type {
                    IndexType::U16 => 2,
                    IndexType::U32 => 4,
                };

                let size = stride * u64::from(indices.count);
                let size_usize = usize::try_from(size).map_err(|_| OutOfMemory)?;

                Ok(Indices {
                    buffer: graphics
                        .create_buffer_static(
                            BufferInfo {
                                align: 255,
                                size,
                                usage: BufferUsage::INDEX,
                            },
                            &data[indices.offset..][..size_usize],
                        )?
                        .into(),
                    offset: 0,
                    index_type: match indices.index_type {
                        IndexType::U16 => IndexType::U16,
                        IndexType::U32 => IndexType::U32,
                    },
                })
            })
            .transpose()?;

        Ok(Mesh::builder()
            .with_bindings(bindings)
            .with_indices_maybe(indices)
            .with_topology(topology)
            .build(count, vertex_count))
    }
}

#[doc(hidden)]
pub struct MeshFile {
    header: MeshFileHeader,
    bytes: Box<[u8]>,
}

#[derive(Debug, thiserror::Error)]
pub enum MeshFileDecodeError {
    #[error("Failed to verify magic number")]
    MagicError,

    #[error("Failed to deserialize magic file header")]
    HeaderError { source: bincode::Error },
}

impl Asset for Mesh {
    type Decoded = MeshFile;
    type DecodeError = MeshFileDecodeError;
    type BuildError = OutOfMemory;
    type Fut = Ready<Result<MeshFile, MeshFileDecodeError>>;

    fn name() -> &'static str {
        "arcana.mesh"
    }

    fn decode(bytes: Box<[u8]>, _loader: &Loader) -> Self::Fut {
        match &*bytes {
            [a, b, c, d, ..] => {
                let magic = u32::from_le_bytes([*a, *b, *c, *d]);
                if magic != MeshFileHeader::MAGIC {
                    tracing::error!(
                        "Mesh blob contains wrong magic number '{:X}'. Expected '{:X}'",
                        magic,
                        MeshFileHeader::MAGIC
                    );
                    return ready(Err(MeshFileDecodeError::MagicError));
                }
            }
            _ => {
                tracing::error!("Mesh blob is too small");
                return ready(Err(MeshFileDecodeError::MagicError));
            }
        }

        match bincode::deserialize::<MeshFileHeader>(&*bytes) {
            Ok(header) => {
                debug_assert_eq!(header.magic, MeshFileHeader::MAGIC);
                ready(Ok(MeshFile { header, bytes }))
            }
            Err(err) => ready(Err(MeshFileDecodeError::HeaderError { source: err })),
        }
    }
}

impl<B> AssetBuild<B> for Mesh
where
    B: BorrowMut<Graphics>,
{
    fn build(decoded: MeshFile, builder: &mut B) -> Result<Self, OutOfMemory> {
        Mesh::build_from_file_data(
            decoded.header.vertex_count,
            &decoded.header.bindings,
            decoded.header.indices.as_ref(),
            decoded.header.topology,
            &decoded.bytes,
            builder.borrow_mut(),
        )
    }
}
