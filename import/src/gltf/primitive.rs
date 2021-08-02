use super::{align_vec, read_accessor, Error};
use arcana_mesh_file::{
    BindingFileHeader, IndexType, IndicesFileHeader, MeshFileHeader, PrimitiveTopology,
    VertexLayout,
};
use byteorder::{ByteOrder, LittleEndian};
use gltf::{
    accessor::{Accessor, DataType, Dimensions},
    Gltf,
};
use goods_treasury_import::Registry;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fs::File,
    io::Write,
    mem::{size_of, size_of_val},
    ops::Range,
    path::Path,
};
use uuid::Uuid;

#[derive(serde::Serialize)]
pub struct Primitive {
    mesh: Uuid,
    material: Option<usize>,
}

pub fn load_primitive(
    prim: gltf::Primitive,
    gltf: &Gltf,
    sources: &HashMap<usize, Box<[u8]>>,
    path: &Path,
    registry: &mut dyn Registry,
) -> eyre::Result<Primitive> {
    let topology = match prim.mode() {
        gltf::mesh::Mode::Points => PrimitiveTopology::PointList,
        gltf::mesh::Mode::Lines => PrimitiveTopology::LineList,
        gltf::mesh::Mode::LineLoop => {
            return Err(Error::UnsupportedTopology {
                unsupported: gltf::mesh::Mode::LineLoop,
            }
            .into());
        }
        gltf::mesh::Mode::LineStrip => PrimitiveTopology::LineStrip,
        gltf::mesh::Mode::Triangles => PrimitiveTopology::TriangleList,
        gltf::mesh::Mode::TriangleStrip => PrimitiveTopology::TriangleStrip,
        gltf::mesh::Mode::TriangleFan => PrimitiveTopology::TriangleFan,
    };

    let mut mesh_data = Vec::new();

    let vertices = load_vertices(gltf, sources, prim.clone(), &mut mesh_data)?;

    let mut count = vertices.count;
    let indices = prim
        .indices()
        .map(|indices| {
            count = indices.count();

            align_vec(&mut mesh_data, 15);

            load_indices(gltf, sources, indices, &mut mesh_data)
        })
        .transpose()?;

    let count = count.try_into().map_err(|_| Error::Overflow)?;
    let vertex_count = vertices.count.try_into().map_err(|_| Error::Overflow)?;

    let mut bindings = Vec::new();

    bindings.push(BindingFileHeader {
        offset: vertices.positions.start,
        layout: VertexLayout::Position3,
    });

    if let Some(normals) = vertices.normals {
        bindings.push(BindingFileHeader {
            offset: normals.start,
            layout: VertexLayout::Normal3,
        });
    }

    if let Some(tangents) = vertices.tangents {
        bindings.push(BindingFileHeader {
            offset: tangents.start,
            layout: VertexLayout::Tangent3,
        });
    }

    if let Some(uvs) = vertices.uvs {
        bindings.push(BindingFileHeader {
            offset: uvs.start,
            layout: VertexLayout::UV,
        });
    }

    if let Some(joints) = vertices.joints {
        bindings.push(BindingFileHeader {
            offset: joints.start,
            layout: VertexLayout::Joints,
        });
    }

    if let Some(weights) = vertices.weights {
        bindings.push(BindingFileHeader {
            offset: weights.start,
            layout: VertexLayout::Weights,
        });
    }

    let indices = match indices {
        None => None,
        Some(IndicesAux::U16(range)) => Some(IndicesFileHeader {
            offset: range.start,
            count,
            index_type: IndexType::U16,
        }),
        Some(IndicesAux::U32(range)) => Some(IndicesFileHeader {
            offset: range.start,
            count,
            index_type: IndexType::U32,
        }),
    };

    let mut header = MeshFileHeader {
        magic: MeshFileHeader::MAGIC,
        vertex_count,
        bindings,
        indices,
        topology,
    };

    let header_size = bincode::serialized_size(&header).unwrap();
    let header_size = usize::try_from(header_size).unwrap();

    for binding in &mut header.bindings {
        binding.offset += header_size;
    }

    if let Some(indices) = &mut header.indices {
        indices.offset += header_size;
    }

    let mesh_path = path.with_extension(format!("gltf.primitive.{}", prim.index()));
    let mut file = File::create(&mesh_path)?;
    bincode::serialize_into(&mut file, &header)?;
    file.write_all(&mesh_data[..])?;

    let uuid = registry.store(&mesh_path, "arcana.mesh", "arcana.mesh", &["gltf"])?;
    let _ = std::fs::remove_file(&mesh_path);

    Ok(Primitive {
        mesh: uuid,
        material: prim.material().index(),
    })
}

fn load_vertex_attribute(
    attribute: VertexAttribute,
    gltf: &Gltf,
    sources: &HashMap<usize, Box<[u8]>>,
    accessor: Accessor,
    output: &mut Vec<u8>,
) -> Result<Range<usize>, Error> {
    if dimensions(attribute)[0] != accessor.dimensions() {
        return Err(Error::UnexpectedDimensions {
            unexpected: accessor.dimensions(),
            expected: dimensions(attribute),
        });
    }

    let (bytes, stride) = read_accessor(accessor.clone(), gltf, sources)?;
    let start = output.len();
    attribute_from_bytes(attribute, accessor.data_type(), bytes, stride, output)?;
    let end = output.len();
    Ok(start..end)
}

enum IndicesAux {
    U16(Range<usize>),
    U32(Range<usize>),
}

fn load_indices(
    gltf: &Gltf,
    sources: &HashMap<usize, Box<[u8]>>,
    accessor: Accessor<'_>,
    output: &mut Vec<u8>,
) -> Result<IndicesAux, Error> {
    if Dimensions::Scalar != accessor.dimensions() {
        return Err(Error::UnexpectedDimensions {
            unexpected: accessor.dimensions(),
            expected: &[Dimensions::Scalar],
        });
    }

    let (bytes, stride) = read_accessor(accessor.clone(), gltf, sources)?;

    // glTF explicitly defines the endianness of binary data as little endian
    match accessor.data_type() {
        DataType::U16 => {
            assert_eq!(size_of::<u16>(), accessor.size());

            let start = output.len();

            if cfg!(target_endian = "little") && stride == size_of::<u16>() {
                // glTF defines all data to be in little endian.
                // If indices are packed and host is little endian
                // they can be copied.
                output.extend_from_slice(bytes);
                Ok(IndicesAux::U16(start..output.len()))
            } else {
                for index in bytes.chunks(stride) {
                    let index = LittleEndian::read_u16(&index[..2]);
                    output.extend(index.to_ne_bytes().iter().copied());
                }
                Ok(IndicesAux::U16(start..output.len()))
            }
        }
        DataType::U32 => {
            assert_eq!(size_of::<u32>(), accessor.size());

            let start = output.len();

            if cfg!(target_endian = "little") && stride == size_of::<u32>() {
                // glTF defines all data to be in little endian.
                // If indices are packed and host is little endian
                // they can be copied.
                output.extend_from_slice(bytes);
                Ok(IndicesAux::U32(start..output.len()))
            } else {
                for index in bytes.chunks(stride) {
                    let index = LittleEndian::read_u32(&index[..4]);
                    output.extend(index.to_ne_bytes().iter().copied());
                }
                Ok(IndicesAux::U32(start..output.len()))
            }
        }
        unexpected => Err(Error::UnexpectedDataType {
            unexpected,
            expected: &[DataType::U16, DataType::U32],
        }),
    }
}

trait GltfDataType: Sized + 'static {
    const DIMENSIONS: Dimensions;
    fn from_bytes(
        data_type: DataType,
        bytes: &[u8],
        stride: usize,
        output: &mut Vec<u8>,
    ) -> Result<(), Error>;
}

#[derive(Clone, Copy)]
enum VertexAttribute {
    Position3,
    Normal3,
    Tangent3,
    UV,
    Joints,
    Weights,
}

fn dimensions(attribute: VertexAttribute) -> &'static [Dimensions] {
    match attribute {
        VertexAttribute::Position3 => &[Dimensions::Vec3],
        VertexAttribute::Normal3 => &[Dimensions::Vec3],
        VertexAttribute::Tangent3 => &[Dimensions::Vec4],
        VertexAttribute::UV => &[Dimensions::Vec2],
        VertexAttribute::Joints => &[Dimensions::Vec4],
        VertexAttribute::Weights => &[Dimensions::Vec4],
    }
}

fn attribute_from_bytes(
    attribute: VertexAttribute,
    data_type: DataType,
    bytes: &[u8],
    stride: usize,
    output: &mut Vec<u8>,
) -> Result<(), Error> {
    match attribute {
        VertexAttribute::Position3 | VertexAttribute::Normal3 => {
            debug_assert!(stride >= size_of::<[f32; 3]>());
            match data_type {
                DataType::F32
                    if cfg!(target_endian = "little") && stride == size_of::<[f32; 3]>() =>
                {
                    output.extend_from_slice(bytes);
                    Ok(())
                }
                DataType::F32 => {
                    for bytes in bytes.chunks(stride) {
                        let mut a = [0f32; 3];
                        LittleEndian::read_f32_into(&bytes[..size_of_val(&a)], &mut a);
                        output.extend_from_slice(bytemuck::bytes_of(&a));
                    }
                    Ok(())
                }
                _ => Err(Error::UnexpectedDataType {
                    unexpected: data_type,
                    expected: &[DataType::F32],
                }),
            }
        }
        VertexAttribute::Tangent3 => {
            debug_assert!(stride >= size_of::<[f32; 4]>());
            match data_type {
                DataType::F32
                    if cfg!(target_endian = "little") && stride == size_of::<[f32; 4]>() =>
                {
                    output.extend_from_slice(bytes);
                    Ok(())
                }
                DataType::F32 => {
                    for bytes in bytes.chunks(stride) {
                        let mut a = [0f32; 4];
                        LittleEndian::read_f32_into(&bytes[..size_of_val(&a)], &mut a);
                        output.extend_from_slice(bytemuck::bytes_of(&a));
                    }
                    Ok(())
                }
                _ => Err(Error::UnexpectedDataType {
                    unexpected: data_type,
                    expected: &[DataType::F32],
                }),
            }
        }
        VertexAttribute::UV => match data_type {
            DataType::F32 if cfg!(target_endian = "little") && stride == size_of::<[f32; 2]>() => {
                output.extend_from_slice(bytes);
                Ok(())
            }
            DataType::F32 => {
                for bytes in bytes.chunks(stride) {
                    let mut a = [0.0; 2];
                    LittleEndian::read_f32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&a));
                }
                Ok(())
            }
            DataType::U8 => {
                debug_assert!(stride >= size_of::<[u8; 2]>());
                for bytes in bytes.chunks(stride) {
                    output.extend_from_slice(bytemuck::bytes_of(&[
                        u8_norm(bytes[0]),
                        u8_norm(bytes[1]),
                    ]));
                }
                Ok(())
            }
            DataType::U16 => {
                debug_assert!(stride >= size_of::<[u16; 2]>());
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 2];
                    LittleEndian::read_u16_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&[u16_norm(a[0]), u16_norm(a[1])]));
                }
                Ok(())
            }
            DataType::U32 => {
                debug_assert!(stride >= size_of::<[u32; 2]>());
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 2];
                    LittleEndian::read_u32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&[u32_norm(a[0]), u32_norm(a[1])]));
                }
                Ok(())
            }
            _ => Err(Error::UnexpectedDataType {
                unexpected: data_type,
                expected: &[DataType::F32],
            }),
        },
        VertexAttribute::Joints => match data_type {
            DataType::U32 if cfg!(target_endian = "little") && stride == size_of::<[u32; 4]>() => {
                output.extend_from_slice(bytes);
                Ok(())
            }
            DataType::U32 => {
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 4];
                    LittleEndian::read_u32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&a));
                }
                Ok(())
            }
            DataType::U8 => {
                debug_assert!(stride >= size_of::<[u8; 4]>());
                for bytes in bytes.chunks(stride) {
                    output.extend_from_slice(bytemuck::bytes_of(&[
                        bytes[0] as u32,
                        bytes[1] as u32,
                        bytes[2] as u32,
                        bytes[3] as u32,
                    ]));
                }
                Ok(())
            }
            DataType::U16 => {
                debug_assert!(stride >= size_of::<[u16; 4]>());
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 4];
                    LittleEndian::read_u16_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&[
                        a[0] as u32,
                        a[1] as u32,
                        a[2] as u32,
                        a[3] as u32,
                    ]));
                }
                Ok(())
            }
            _ => Err(Error::UnexpectedDataType {
                unexpected: data_type,
                expected: &[DataType::F32],
            }),
        },
        VertexAttribute::Weights => match data_type {
            DataType::F32 if cfg!(target_endian = "little") && stride == size_of::<[f32; 4]>() => {
                output.extend_from_slice(bytes);
                Ok(())
            }
            DataType::F32 => {
                for bytes in bytes.chunks(stride) {
                    let mut a = [0.0; 4];
                    LittleEndian::read_f32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&a));
                }
                Ok(())
            }
            DataType::U8 => {
                debug_assert!(stride >= size_of::<[u8; 4]>());
                for bytes in bytes.chunks(stride) {
                    output.extend_from_slice(bytemuck::bytes_of(&[
                        u8_norm(bytes[0]),
                        u8_norm(bytes[1]),
                        u8_norm(bytes[2]),
                        u8_norm(bytes[3]),
                    ]));
                }
                Ok(())
            }
            DataType::U16 => {
                debug_assert!(stride >= size_of::<[u16; 4]>());
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 4];
                    LittleEndian::read_u16_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&[
                        u16_norm(a[0]),
                        u16_norm(a[1]),
                        u16_norm(a[2]),
                        u16_norm(a[3]),
                    ]));
                }
                Ok(())
            }
            DataType::U32 => {
                debug_assert!(stride >= size_of::<[u32; 4]>());
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 4];
                    LittleEndian::read_u32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&[
                        u32_norm(a[0]),
                        u32_norm(a[1]),
                        u32_norm(a[2]),
                        u32_norm(a[3]),
                    ]));
                }
                Ok(())
            }
            _ => Err(Error::UnexpectedDataType {
                unexpected: data_type,
                expected: &[DataType::F32],
            }),
        },
    }
}

struct Vertices {
    positions: Range<usize>,
    normals: Option<Range<usize>>,
    tangents: Option<Range<usize>>,
    uvs: Option<Range<usize>>,
    joints: Option<Range<usize>>,
    weights: Option<Range<usize>>,
    count: usize,
}

fn load_vertices(
    gltf: &Gltf,
    sources: &HashMap<usize, Box<[u8]>>,
    primitive: gltf::mesh::Primitive<'_>,
    output: &mut Vec<u8>,
) -> Result<Vertices, Error> {
    let position = primitive
        .get(&gltf::Semantic::Positions)
        .ok_or(Error::MissingPositionAttribute)?;

    let mut count = position.count();
    align_vec(output, 15);
    let positions =
        load_vertex_attribute(VertexAttribute::Position3, gltf, sources, position, output)?;

    align_vec(output, 15);
    let normals = primitive
        .get(&gltf::Semantic::Normals)
        .map(|accessor| {
            count = count.min(accessor.count());
            load_vertex_attribute(VertexAttribute::Normal3, gltf, sources, accessor, output)
        })
        .transpose()?;

    align_vec(output, 15);
    let tangents = primitive
        .get(&gltf::Semantic::Tangents)
        .map(|accessor| {
            count = count.min(accessor.count());
            load_vertex_attribute(VertexAttribute::Tangent3, gltf, sources, accessor, output)
        })
        .transpose()?;

    align_vec(output, 15);
    let uvs = primitive
        .get(&gltf::Semantic::TexCoords(0))
        .map(|accessor| {
            count = count.min(accessor.count());
            load_vertex_attribute(VertexAttribute::UV, gltf, sources, accessor, output)
        })
        .transpose()?;

    align_vec(output, 15);
    let joints = primitive
        .get(&gltf::Semantic::Joints(0))
        .map(|accessor| {
            count = count.min(accessor.count());
            load_vertex_attribute(VertexAttribute::Joints, gltf, sources, accessor, output)
        })
        .transpose()?;

    align_vec(output, 15);
    let weights = primitive
        .get(&gltf::Semantic::Weights(0))
        .map(|accessor| {
            count = count.min(accessor.count());
            load_vertex_attribute(VertexAttribute::Weights, gltf, sources, accessor, output)
        })
        .transpose()?;

    Ok(Vertices {
        positions,
        normals,
        tangents,
        uvs,
        joints,
        weights,
        count,
    })
}

fn u8_norm(v: u8) -> f32 {
    const U8_NORM: f32 = 1.0 / u8::MAX as f32;
    v as f32 * U8_NORM
}

fn u16_norm(v: u16) -> f32 {
    const U16_NORM: f32 = 1.0 / u16::MAX as f32;
    v as f32 * U16_NORM
}

fn u32_norm(v: u32) -> f32 {
    const U32_NORM: f32 = 1.0 / u32::MAX as f32;
    v as f32 * U32_NORM
}
