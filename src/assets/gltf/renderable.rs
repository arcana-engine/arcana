use {
    super::{
        align_vec, read_accessor, GltfBuildContext, GltfDataType, GltfDecoded, GltfLoadingError,
        GltfRenderable,
    },
    crate::graphics::{
        Binding, Indices, Joints, MeshBuilder, Normal3, Position3, Tangent3, VertexType, Weights,
        UV,
    },
    byteorder::{ByteOrder as _, LittleEndian},
    gltf::accessor::{Accessor, DataType, Dimensions},
    sierra::*,
    std::{
        convert::{TryFrom as _, TryInto as _},
        mem::{size_of, size_of_val},
        ops::Range,
    },
};

impl GltfBuildContext<'_> {
    pub fn create_primitive(
        &mut self,
        prim: gltf::Primitive,
    ) -> Result<GltfRenderable, GltfLoadingError> {
        let topology = match prim.mode() {
            gltf::mesh::Mode::Points => PrimitiveTopology::PointList,
            gltf::mesh::Mode::Lines => PrimitiveTopology::LineList,
            gltf::mesh::Mode::LineLoop => {
                return Err(GltfLoadingError::UnsupportedTopology {
                    unsupported: gltf::mesh::Mode::LineLoop,
                });
            }
            gltf::mesh::Mode::LineStrip => PrimitiveTopology::LineStrip,
            gltf::mesh::Mode::Triangles => PrimitiveTopology::TriangleList,
            gltf::mesh::Mode::TriangleStrip => PrimitiveTopology::TriangleStrip,
            gltf::mesh::Mode::TriangleFan => PrimitiveTopology::TriangleFan,
        };

        let mut loaded_data = Vec::new();

        let vertices = load_vertices(self.decoded, prim.clone(), &mut loaded_data)?;

        let mut count = vertices.count;
        let indices = prim
            .indices()
            .map(|indices| {
                count = indices.count();

                align_vec(&mut loaded_data, 15);

                load_indices(self.decoded, indices, &mut loaded_data)
            })
            .transpose()?;

        let count = count.try_into().map_err(|_| OutOfMemory)?;
        let vertex_count = vertices.count.try_into().map_err(|_| OutOfMemory)?;

        let buffer = self.graphics.create_fast_buffer_static(
            BufferInfo {
                align: 255,
                size: u64::try_from(loaded_data.len()).map_err(|_| OutOfMemory)?,
                usage: self.decoded.config.mesh_indices_usage
                    | self.decoded.config.mesh_vertices_usage,
            },
            &loaded_data,
        )?;

        let mut bindings = Vec::new();

        bindings.push(Binding {
            buffer: buffer.clone(),
            offset: vertices.positions.start as u64,
            layout: Position3::layout(),
        });

        if let Some(normals) = vertices.normals {
            bindings.push(Binding {
                buffer: buffer.clone(),
                offset: normals.start as u64,
                layout: Normal3::layout(),
            });
        }

        if let Some(tangents) = vertices.tangents {
            bindings.push(Binding {
                buffer: buffer.clone(),
                offset: tangents.start as u64,
                layout: Tangent3::layout(),
            });
        }

        if let Some(uvs) = vertices.uvs {
            bindings.push(Binding {
                buffer: buffer.clone(),
                offset: uvs.start as u64,
                layout: UV::layout(),
            });
        }

        if let Some(joints) = vertices.joints {
            bindings.push(Binding {
                buffer: buffer.clone(),
                offset: joints.start as u64,
                layout: Joints::layout(),
            });
        }

        if let Some(weights) = vertices.weights {
            bindings.push(Binding {
                buffer: buffer.clone(),
                offset: weights.start as u64,
                layout: Weights::layout(),
            });
        }

        let indices = match indices {
            None => None,
            Some(IndicesAux::U16(range)) => Some(Indices {
                buffer: buffer.clone(),
                offset: range.start as u64,
                index_type: IndexType::U16,
            }),
            Some(IndicesAux::U32(range)) => Some(Indices {
                buffer: buffer.clone(),
                offset: range.start as u64,
                index_type: IndexType::U32,
            }),
        };

        let mesh = MeshBuilder {
            bindings,
            indices,
            topology,
        };

        let mesh = mesh.build(count, vertex_count);
        let material = self.get_material(prim.material())?;

        Ok(GltfRenderable { mesh, material })
    }
}

enum IndicesAux {
    U16(Range<usize>),
    U32(Range<usize>),
}

fn load_indices(
    repr: &GltfDecoded,
    accessor: Accessor<'_>,
    output: &mut Vec<u8>,
) -> Result<IndicesAux, GltfLoadingError> {
    if Dimensions::Scalar != accessor.dimensions() {
        return Err(GltfLoadingError::UnexpectedDimensions {
            unexpected: accessor.dimensions(),
            expected: &[Dimensions::Scalar],
        });
    }

    let (bytes, stride) = read_accessor(accessor.clone(), repr)?;

    // glTF explicitly defines the endianness of binary data as little endian
    match accessor.data_type() {
        DataType::U16 => {
            assert_eq!(size_of::<u16>(), accessor.size());

            let start = output.len();

            for index in bytes.chunks(stride) {
                let index = LittleEndian::read_u16(&index[..2]);
                output.extend((index as u32).to_ne_bytes().iter().copied());
            }
            Ok(IndicesAux::U32(start..output.len()))
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
        unexpected => Err(GltfLoadingError::UnexpectedDataType {
            unexpected,
            expected: &[DataType::U16, DataType::U32],
        }),
    }
}

impl GltfDataType for Position3 {
    const DIMENSIONS: Dimensions = Dimensions::Vec3;

    fn from_bytes(
        data_type: DataType,
        bytes: &[u8],
        stride: usize,
        output: &mut Vec<u8>,
    ) -> Result<(), GltfLoadingError> {
        debug_assert!(stride >= size_of::<Self>());
        match data_type {
            DataType::F32 if cfg!(target_endian = "little") && stride == size_of::<Self>() => {
                output.extend_from_slice(bytes);
                Ok(())
            }
            DataType::F32 => {
                for bytes in bytes.chunks(stride) {
                    let mut a = [0.0; 3];
                    LittleEndian::read_f32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&Position3(a)));
                }
                Ok(())
            }
            _ => Err(GltfLoadingError::UnexpectedDataType {
                unexpected: data_type,
                expected: &[DataType::F32],
            }),
        }
    }
}

impl GltfDataType for Normal3 {
    const DIMENSIONS: Dimensions = Dimensions::Vec3;

    fn from_bytes(
        data_type: DataType,
        bytes: &[u8],
        stride: usize,
        output: &mut Vec<u8>,
    ) -> Result<(), GltfLoadingError> {
        debug_assert!(stride >= size_of::<Self>());
        match data_type {
            DataType::F32 if cfg!(target_endian = "little") && stride == size_of::<Self>() => {
                output.extend_from_slice(bytes);
                Ok(())
            }
            DataType::F32 => {
                for bytes in bytes.chunks(stride) {
                    let mut a = [0.0; 3];
                    LittleEndian::read_f32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&Normal3(a)));
                }
                Ok(())
            }
            _ => Err(GltfLoadingError::UnexpectedDataType {
                unexpected: data_type,
                expected: &[DataType::F32],
            }),
        }
    }
}

impl GltfDataType for Tangent3 {
    const DIMENSIONS: Dimensions = Dimensions::Vec4;

    fn from_bytes(
        data_type: DataType,
        bytes: &[u8],
        stride: usize,
        output: &mut Vec<u8>,
    ) -> Result<(), GltfLoadingError> {
        debug_assert!(stride >= size_of::<Self>());
        match data_type {
            DataType::F32 if cfg!(target_endian = "little") && stride == size_of::<Self>() => {
                output.extend_from_slice(bytes);
                Ok(())
            }
            DataType::F32 => {
                for bytes in bytes.chunks(stride) {
                    let mut a = [0.0; 4];
                    LittleEndian::read_f32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&Tangent3(a)));
                }
                Ok(())
            }
            _ => Err(GltfLoadingError::UnexpectedDataType {
                unexpected: data_type,
                expected: &[DataType::F32],
            }),
        }
    }
}

impl GltfDataType for UV {
    const DIMENSIONS: Dimensions = Dimensions::Vec2;

    fn from_bytes(
        data_type: DataType,
        bytes: &[u8],
        stride: usize,
        output: &mut Vec<u8>,
    ) -> Result<(), GltfLoadingError> {
        match data_type {
            DataType::F32 if cfg!(target_endian = "little") && stride == size_of::<Self>() => {
                output.extend_from_slice(bytes);
                Ok(())
            }
            DataType::F32 => {
                for bytes in bytes.chunks(stride) {
                    let mut a = [0.0; 2];
                    LittleEndian::read_f32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&UV(a)));
                }
                Ok(())
            }
            DataType::U8 => {
                debug_assert!(stride >= size_of::<[u8; 2]>());
                for bytes in bytes.chunks(stride) {
                    output.extend_from_slice(bytemuck::bytes_of(&UV([
                        u8_norm(bytes[0]),
                        u8_norm(bytes[1]),
                    ])));
                }
                Ok(())
            }
            DataType::U16 => {
                debug_assert!(stride >= size_of::<[u16; 2]>());
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 2];
                    LittleEndian::read_u16_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&UV([
                        u16_norm(a[0]),
                        u16_norm(a[1]),
                    ])));
                }
                Ok(())
            }
            DataType::U32 => {
                debug_assert!(stride >= size_of::<[u32; 2]>());
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 2];
                    LittleEndian::read_u32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&UV([
                        u32_norm(a[0]),
                        u32_norm(a[1]),
                    ])));
                }
                Ok(())
            }
            _ => Err(GltfLoadingError::UnexpectedDataType {
                unexpected: data_type,
                expected: &[DataType::F32],
            }),
        }
    }
}

impl GltfDataType for Joints {
    const DIMENSIONS: Dimensions = Dimensions::Vec4;

    fn from_bytes(
        data_type: DataType,
        bytes: &[u8],
        stride: usize,
        output: &mut Vec<u8>,
    ) -> Result<(), GltfLoadingError> {
        match data_type {
            DataType::U32 if cfg!(target_endian = "little") && stride == size_of::<Self>() => {
                output.extend_from_slice(bytes);
                Ok(())
            }
            DataType::U32 => {
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 4];
                    LittleEndian::read_u32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&Joints(a)));
                }
                Ok(())
            }
            DataType::U8 => {
                debug_assert!(stride >= size_of::<[u8; 4]>());
                for bytes in bytes.chunks(stride) {
                    output.extend_from_slice(bytemuck::bytes_of(&Joints([
                        bytes[0] as u32,
                        bytes[1] as u32,
                        bytes[2] as u32,
                        bytes[3] as u32,
                    ])));
                }
                Ok(())
            }
            DataType::U16 => {
                debug_assert!(stride >= size_of::<[u16; 4]>());
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 4];
                    LittleEndian::read_u16_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&Joints([
                        a[0] as u32,
                        a[1] as u32,
                        a[2] as u32,
                        a[3] as u32,
                    ])));
                }
                Ok(())
            }
            _ => Err(GltfLoadingError::UnexpectedDataType {
                unexpected: data_type,
                expected: &[DataType::F32],
            }),
        }
    }
}

impl GltfDataType for Weights {
    const DIMENSIONS: Dimensions = Dimensions::Vec4;

    fn from_bytes(
        data_type: DataType,
        bytes: &[u8],
        stride: usize,
        output: &mut Vec<u8>,
    ) -> Result<(), GltfLoadingError> {
        match data_type {
            DataType::F32 if cfg!(target_endian = "little") && stride == size_of::<Self>() => {
                output.extend_from_slice(bytes);
                Ok(())
            }
            DataType::F32 => {
                for bytes in bytes.chunks(stride) {
                    let mut a = [0.0; 4];
                    LittleEndian::read_f32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&Weights(a)));
                }
                Ok(())
            }
            DataType::U8 => {
                debug_assert!(stride >= size_of::<[u8; 4]>());
                for bytes in bytes.chunks(stride) {
                    output.extend_from_slice(bytemuck::bytes_of(&Weights([
                        u8_norm(bytes[0]),
                        u8_norm(bytes[1]),
                        u8_norm(bytes[2]),
                        u8_norm(bytes[3]),
                    ])));
                }
                Ok(())
            }
            DataType::U16 => {
                debug_assert!(stride >= size_of::<[u16; 4]>());
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 4];
                    LittleEndian::read_u16_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&Weights([
                        u16_norm(a[0]),
                        u16_norm(a[1]),
                        u16_norm(a[2]),
                        u16_norm(a[3]),
                    ])));
                }
                Ok(())
            }
            DataType::U32 => {
                debug_assert!(stride >= size_of::<[u32; 4]>());
                for bytes in bytes.chunks(stride) {
                    let mut a = [0; 4];
                    LittleEndian::read_u32_into(&bytes[..size_of_val(&a)], &mut a);
                    output.extend_from_slice(bytemuck::bytes_of(&Weights([
                        u32_norm(a[0]),
                        u32_norm(a[1]),
                        u32_norm(a[2]),
                        u32_norm(a[3]),
                    ])));
                }
                Ok(())
            }
            _ => Err(GltfLoadingError::UnexpectedDataType {
                unexpected: data_type,
                expected: &[DataType::F32],
            }),
        }
    }
}

struct ReadVertexAttribute<'a> {
    bytes: &'a [u8],
    stride: usize,
    data_type: DataType,
    count: usize,
}

fn load_vertex_attribute<'a, V: GltfDataType>(
    decoded: &'a GltfDecoded,
    accessor: Accessor<'_>,
    output: &mut Vec<u8>,
) -> Result<Range<usize>, GltfLoadingError> {
    if V::DIMENSIONS != accessor.dimensions() {
        return Err(GltfLoadingError::UnexpectedDimensions {
            unexpected: accessor.dimensions(),
            expected: &[V::DIMENSIONS],
        });
    }

    let (bytes, stride) = read_accessor(accessor.clone(), decoded)?;
    let start = output.len();
    V::from_bytes(accessor.data_type(), bytes, stride, output)?;
    let end = output.len();
    Ok(start..end)
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
    decoded: &GltfDecoded,
    primitive: gltf::mesh::Primitive<'_>,
    output: &mut Vec<u8>,
) -> Result<Vertices, GltfLoadingError> {
    let position = primitive
        .get(&gltf::Semantic::Positions)
        .ok_or(GltfLoadingError::MissingPositionAttribute)?;

    let mut count = position.count();
    align_vec(output, 15);
    let positions = load_vertex_attribute::<Position3>(decoded, position, output)?;

    align_vec(output, 15);
    let normals = primitive
        .get(&gltf::Semantic::Normals)
        .map(|accessor| {
            count = count.min(accessor.count());
            load_vertex_attribute::<Normal3>(decoded, accessor, output)
        })
        .transpose()?;

    align_vec(output, 15);
    let tangents = primitive
        .get(&gltf::Semantic::Tangents)
        .map(|accessor| {
            count = count.min(accessor.count());
            load_vertex_attribute::<Tangent3>(decoded, accessor, output)
        })
        .transpose()?;

    align_vec(output, 15);
    let uvs = primitive
        .get(&gltf::Semantic::TexCoords(0))
        .map(|accessor| {
            count = count.min(accessor.count());
            load_vertex_attribute::<UV>(decoded, accessor, output)
        })
        .transpose()?;

    align_vec(output, 15);
    let joints = primitive
        .get(&gltf::Semantic::Joints(0))
        .map(|accessor| {
            count = count.min(accessor.count());
            load_vertex_attribute::<Joints>(decoded, accessor, output)
        })
        .transpose()?;

    align_vec(output, 15);
    let weights = primitive
        .get(&gltf::Semantic::Weights(0))
        .map(|accessor| {
            count = count.min(accessor.count());
            load_vertex_attribute::<Weights>(decoded, accessor, output)
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
