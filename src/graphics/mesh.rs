use {
    super::{
        vertex::{
            Color, Joints, Normal3, Position3, PositionNormal3, PositionNormal3Color,
            PositionNormal3UV, PositionNormalTangent3, PositionNormalTangent3Color,
            PositionNormalTangent3UV, Semantics, Skin, Tangent3, VertexLayout, VertexLocation,
            VertexType, Weights, UV,
        },
        Graphics,
    },
    arcana_mesh_file::{MeshFile, MeshFileHeader},
    bumpalo::{collections::Vec as BVec, Bump},
    bytemuck::cast_slice,
    goods::{Asset, AssetBuild, Loader},
    sierra::{
        AccelerationStructure, AccelerationStructureBuildFlags,
        AccelerationStructureBuildGeometryInfo, AccelerationStructureGeometry,
        AccelerationStructureGeometryInfo, AccelerationStructureInfo, AccelerationStructureLevel,
        Buffer, BufferInfo, BufferRange, BufferUsage, Device, Encoder, Format, GeometryFlags,
        IndexData, IndexType, OutOfMemory, PrimitiveTopology, RenderPassEncoder, VertexInputRate,
    },
    std::{
        borrow::{BorrowMut, Cow},
        convert::TryFrom as _,
        future::{ready, Ready},
        mem::size_of_val,
        ops::Range,
        sync::Arc,
    },
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Binding {
    pub buffer: Buffer,
    pub offset: u64,
    pub layout: VertexLayout,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Indices {
    pub buffer: Buffer,
    pub offset: u64,
    pub index_type: IndexType,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct MeshBuilder {
    pub bindings: Vec<Binding>,
    pub indices: Option<Indices>,
    pub topology: PrimitiveTopology,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self::with_topology(PrimitiveTopology::TriangleList)
    }

    pub fn with_topology(topology: PrimitiveTopology) -> Self {
        MeshBuilder {
            bindings: Vec::new(),
            indices: None,
            topology,
        }
    }

    pub fn with_binding(mut self, buffer: Buffer, offset: u64, layout: VertexLayout) -> Self {
        if layout.rate == VertexInputRate::Instance {
            tracing::warn!("Instance-rate attribute are not suitable for Mesh");
        }
        self.add_binding(buffer, offset, layout);
        self
    }

    pub fn add_binding(&mut self, buffer: Buffer, offset: u64, layout: VertexLayout) -> &mut Self {
        self.bindings.push(Binding {
            buffer,
            offset,
            layout,
        });

        self
    }

    pub fn with_indices(mut self, buffer: Buffer, offset: u64, index_type: IndexType) -> Self {
        self.set_indices(buffer, offset, index_type);

        self
    }

    pub fn set_indices(&mut self, buffer: Buffer, offset: u64, index_type: IndexType) -> &mut Self {
        self.indices = Some(Indices {
            buffer,
            offset,
            index_type,
        });

        self
    }

    pub fn build(self, count: u32, vertex_count: u32) -> Mesh {
        Mesh {
            bindings: self.bindings.into(),
            indices: self.indices,
            topology: self.topology,
            count,
            vertex_count,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Mesh {
    bindings: Arc<[Binding]>,
    indices: Option<Indices>,
    count: u32,
    vertex_count: u32,
    topology: PrimitiveTopology,
}

impl Mesh {
    pub fn builder() -> MeshBuilder {
        MeshBuilder::new()
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    pub fn bindings(&self) -> &[Binding] {
        &*self.bindings
    }

    pub fn indices(&self) -> Option<&Indices> {
        self.indices.as_ref()
    }

    pub fn build_triangles_blas<'a>(
        &self,
        encoder: &mut Encoder<'a>,
        device: &Device,
        bump: &'a Bump,
    ) -> Result<AccelerationStructure, OutOfMemory> {
        assert_eq!(self.topology, PrimitiveTopology::TriangleList);

        let (pos_binding, pos_location) = self
            .bindings
            .iter()
            .filter_map(|binding| {
                binding
                    .layout
                    .locations
                    .iter()
                    .find(|&attr| attr.semantics == Semantics::Position3)
                    .map(move |location| (binding, location))
            })
            .next()
            .expect("Cannot create acceleration structure for mesh without position attribute");

        build_triangles_blas(
            self.indices.as_ref(),
            pos_binding,
            pos_location,
            self.count,
            self.vertex_count,
            encoder,
            device,
            bump,
        )
    }

    pub fn build_pose_triangles_blas<'a>(
        &self,
        pose: &PoseMesh,
        encoder: &mut Encoder<'a>,
        device: &Device,
        bump: &'a Bump,
    ) -> Result<AccelerationStructure, OutOfMemory> {
        assert_eq!(self.topology, PrimitiveTopology::TriangleList);

        let (pos_binding, pos_location) = pose
            .bindings
            .iter()
            .filter_map(|binding| {
                binding
                    .layout
                    .locations
                    .iter()
                    .find(|&attr| attr.semantics == Semantics::Position3)
                    .map(move |location| (binding, location))
            })
            .next()
            .expect("Cannot create acceleration structure for mesh without position attribute");

        build_triangles_blas(
            self.indices.as_ref(),
            pos_binding,
            pos_location,
            self.count,
            self.vertex_count,
            encoder,
            device,
            bump,
        )
    }

    pub fn draw<'a>(
        &'a self,
        instances: Range<u32>,
        layouts: &[VertexLayout],
        encoder: &mut RenderPassEncoder<'_, 'a>,
        bump: &'a Bump,
    ) -> bool {
        let mut to_bind = BVec::with_capacity_in(self.bindings.len(), bump);

        'outer: for layout in layouts {
            for binding in &*self.bindings {
                if binding.layout == *layout {
                    to_bind.push((&binding.buffer, binding.offset));
                    continue 'outer;
                }
            }

            tracing::warn!(
                "Cannot find vertex bindings for requested vertex layout {:#?}",
                layout
            );
            return false;
        }

        encoder.bind_vertex_buffers(0, &to_bind);

        if let Some(indices) = &self.indices {
            encoder.bind_index_buffer(&indices.buffer, indices.offset, indices.index_type);

            encoder.draw_indexed(0..self.count, 0, instances);
        } else {
            encoder.draw(0..self.count, instances);
        }

        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BindingData<'a> {
    #[serde(with = "serde_bytes", borrow = "'a")]
    pub data: Cow<'a, [u8]>,
    pub layout: VertexLayout,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IndicesData<'a> {
    #[serde(with = "serde_bytes", borrow = "'a")]
    pub data: Cow<'a, [u8]>,
    pub index_type: IndexType,
}

impl<'a, const N: usize> From<&'a [u16; N]> for IndicesData<'a> {
    fn from(indices: &'a [u16; N]) -> Self {
        IndicesData {
            data: bytemuck::cast_slice(indices).into(),
            index_type: IndexType::U16,
        }
    }
}

impl<'a, const N: usize> From<&'a [u32; N]> for IndicesData<'a> {
    fn from(indices: &'a [u32; N]) -> Self {
        IndicesData {
            data: unsafe {
                std::slice::from_raw_parts(indices.as_ptr() as *const u8, size_of_val(indices))
            }
            .into(),
            index_type: IndexType::U32,
        }
    }
}

impl<'a> From<&'a [u16]> for IndicesData<'a> {
    fn from(indices: &'a [u16]) -> Self {
        IndicesData {
            data: unsafe {
                std::slice::from_raw_parts(indices.as_ptr() as *const u8, size_of_val(indices))
            }
            .into(),
            index_type: IndexType::U16,
        }
    }
}

impl<'a> From<&'a [u32]> for IndicesData<'a> {
    fn from(indices: &'a [u32]) -> Self {
        IndicesData {
            data: unsafe {
                std::slice::from_raw_parts(indices.as_ptr() as *const u8, size_of_val(indices))
            }
            .into(),
            index_type: IndexType::U32,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MeshData<'a> {
    #[serde(skip_serializing_if = "Vec::is_empty", default, borrow = "'a")]
    pub bindings: Vec<BindingData<'a>>,
    #[serde(skip_serializing_if = "Option::is_none", default, borrow = "'a")]
    pub indices: Option<IndicesData<'a>>,
    #[serde(
        skip_serializing_if = "topology_is_triangles",
        default = "topology_triangles"
    )]
    pub topology: PrimitiveTopology,
}

impl MeshData<'_> {
    pub fn new() -> Self {
        MeshData::with_topology(PrimitiveTopology::TriangleList)
    }

    pub fn with_topology(topology: PrimitiveTopology) -> Self {
        MeshData {
            bindings: Vec::new(),
            indices: None,
            topology,
        }
    }
}

impl<'a> MeshData<'a> {
    pub fn add_binding<V>(&mut self, vertices: &'a [V]) -> &mut Self
    where
        V: VertexType,
    {
        self.bindings.push(BindingData {
            data: Cow::Borrowed(cast_slice(vertices)),
            layout: V::layout(),
        });
        self
    }

    pub fn with_binding<V>(mut self, vertices: &'a [V]) -> Self
    where
        V: VertexType,
    {
        self.add_binding(vertices);
        self
    }

    pub fn set_indices<I>(&mut self, indices: I) -> &mut Self
    where
        I: Into<IndicesData<'a>>,
    {
        self.indices = Some(indices.into());
        self
    }

    pub fn with_indices<I>(mut self, indices: I) -> Self
    where
        I: Into<IndicesData<'a>>,
    {
        self.set_indices(indices);
        self
    }
}

impl MeshData<'_> {
    pub fn build(
        &self,
        graphics: &mut Graphics,
        vertices_usage: BufferUsage,
        indices_usage: BufferUsage,
    ) -> Result<Mesh, OutOfMemory> {
        let mut min_vertex_count = !0u32;

        let bindings: Arc<[Binding]> = self
            .bindings
            .iter()
            .map(|binding| -> Result<_, OutOfMemory> {
                let vertex_count = u64::try_from(binding.data.len()).map_err(|_| OutOfMemory)?
                    / u64::from(binding.layout.stride);

                let vertex_count = u32::try_from(vertex_count).map_err(|_| OutOfMemory)?;

                min_vertex_count = min_vertex_count.min(vertex_count);

                Ok(Binding {
                    buffer: graphics
                        .create_buffer_static(
                            BufferInfo {
                                align: 255,
                                size: u64::try_from(binding.data.len()).map_err(|_| OutOfMemory)?,
                                usage: vertices_usage,
                            },
                            &binding.data,
                        )?
                        .into(),
                    offset: 0,
                    layout: binding.layout.clone(),
                })
            })
            .collect::<Result<_, _>>()?;

        let mut count = min_vertex_count;

        let indices = self
            .indices
            .as_ref()
            .map(|indices| -> Result<_, OutOfMemory> {
                let index_count = u64::try_from(indices.data.len()).map_err(|_| OutOfMemory)?
                    / u64::from(indices.index_type.size());

                count = u32::try_from(index_count).map_err(|_| OutOfMemory)?;

                Ok(Indices {
                    buffer: graphics
                        .create_buffer_static(
                            BufferInfo {
                                align: 255,
                                size: u64::try_from(indices.data.len()).map_err(|_| OutOfMemory)?,
                                usage: indices_usage,
                            },
                            &indices.data,
                        )?
                        .into(),
                    offset: 0,
                    index_type: indices.index_type,
                })
            })
            .transpose()?;

        Ok(Mesh {
            bindings,
            indices,
            topology: self.topology,
            count,
            vertex_count: min_vertex_count,
        })
    }

    pub fn build_for_raster(&self, graphics: &mut Graphics) -> Result<Mesh, OutOfMemory> {
        self.build(graphics, BufferUsage::VERTEX, BufferUsage::INDEX)
    }

    pub fn build_for_blas(&self, graphics: &mut Graphics) -> Result<Mesh, OutOfMemory> {
        self.build(
            graphics,
            BufferUsage::ACCELERATION_STRUCTURE_BUILD_INPUT | BufferUsage::STORAGE,
            BufferUsage::ACCELERATION_STRUCTURE_BUILD_INPUT | BufferUsage::STORAGE,
        )
    }
}

fn topology_is_triangles(topology: &PrimitiveTopology) -> bool {
    *topology == PrimitiveTopology::TriangleList
}

fn topology_triangles() -> PrimitiveTopology {
    PrimitiveTopology::TriangleList
}

#[cfg(feature = "genmesh")]
impl Mesh {
    pub fn cube<V>(
        extent: na::Vector3<f32>,
        usage: BufferUsage,
        cx: &mut Graphics,
        index_type: IndexType,
        vertex: impl Fn(genmesh::Vertex) -> V,
    ) -> Result<Self, OutOfMemory>
    where
        V: VertexType,
    {
        Self::from_generator(
            &genmesh::generators::Cube::new(),
            usage,
            cx,
            index_type,
            vertex,
        )
    }

    pub fn cube_pos(
        extent: na::Vector3<f32>,
        usage: BufferUsage,
        cx: &mut Graphics,
        index_type: IndexType,
    ) -> Result<Self, OutOfMemory> {
        Self::from_generator_pos(&genmesh::generators::Cube::new(), usage, cx, index_type)
    }

    pub fn cube_pos_norm(
        extent: na::Vector3<f32>,
        usage: BufferUsage,
        cx: &mut Graphics,
        index_type: IndexType,
    ) -> Result<Self, OutOfMemory> {
        Self::from_generator_pos_norm(&genmesh::generators::Cube::new(), usage, cx, index_type)
    }

    pub fn cube_pos_norm_fixed_color(
        extent: na::Vector3<f32>,
        usage: BufferUsage,
        cx: &mut Graphics,
        index_type: IndexType,
        color: Color,
    ) -> Result<Self, OutOfMemory> {
        Self::from_generator_pos_norm_fixed_color(
            &genmesh::generators::Cube::new(),
            usage,
            cx,
            index_type,
            color,
        )
    }

    pub fn from_generator_pos<G>(
        generator: &G,
        usage: BufferUsage,
        cx: &mut Graphics,
        index_type: IndexType,
    ) -> Result<Self, OutOfMemory>
    where
        G: genmesh::generators::SharedVertex<genmesh::Vertex>
            + genmesh::generators::IndexedPolygon<genmesh::Quad<usize>>,
    {
        Self::from_generator(generator, usage, cx, index_type, |v| {
            Position3(v.pos.into())
        })
    }

    pub fn from_generator_pos_norm<G>(
        generator: &G,
        usage: BufferUsage,
        cx: &mut Graphics,
        index_type: IndexType,
    ) -> Result<Self, OutOfMemory>
    where
        G: genmesh::generators::SharedVertex<genmesh::Vertex>
            + genmesh::generators::IndexedPolygon<genmesh::Quad<usize>>,
    {
        Self::from_generator(generator, usage, cx, index_type, |v| PositionNormal3 {
            position: Position3(v.pos.into()),
            normal: Normal3(v.normal.into()),
        })
    }

    pub fn from_generator_pos_norm_fixed_color<G>(
        generator: &G,
        usage: BufferUsage,
        cx: &mut Graphics,
        index_type: IndexType,
        color: Color,
    ) -> Result<Self, OutOfMemory>
    where
        G: genmesh::generators::SharedVertex<genmesh::Vertex>
            + genmesh::generators::IndexedPolygon<genmesh::Quad<usize>>,
    {
        Self::from_generator(generator, usage, cx, index_type, |v| PositionNormal3Color {
            position: Position3(v.pos.into()),
            normal: Normal3(v.pos.into()),
            color,
        })
    }

    pub fn from_generator<G, V, P>(
        generator: &G,
        usage: BufferUsage,
        cx: &mut Graphics,
        index_type: IndexType,
        vertex: impl Fn(genmesh::Vertex) -> V,
    ) -> Result<Self, OutOfMemory>
    where
        G: genmesh::generators::SharedVertex<genmesh::Vertex>
            + genmesh::generators::IndexedPolygon<P>,
        V: VertexType,
        P: genmesh::EmitTriangles<Vertex = usize>,
    {
        assert_eq!(size_of::<V>(), usize::try_from(V::layout().stride).unwrap());

        let vertices: Vec<_> = generator.shared_vertex_iter().map(vertex).collect();

        let vertices_size = size_of_val(&vertices[..]);

        let indices_offset = ((vertices_size - 1) | 15) + 1;

        let mut data;

        let vertex_count = u32::try_from(vertices.len()).map_err(|_| OutOfMemory)?;

        let index_count;

        let align_data_len = |data_len: usize| ((data_len - 1) | 15) + 1;

        match index_type {
            IndexType::U16 => {
                let indices: Vec<_> = generator
                    .indexed_polygon_iter()
                    .flat_map(|polygon| {
                        let mut indices = Vec::new();

                        polygon.emit_triangles(|triangle| {
                            indices.push(triangle.x);

                            indices.push(triangle.y);

                            indices.push(triangle.z);
                        });

                        indices
                    })
                    .map(|index| u16::try_from(index).unwrap())
                    .collect();

                index_count = u32::try_from(indices.len()).map_err(|_| OutOfMemory)?;

                let indices_size = size_of_val(&indices[..]);

                data = vec![0u8; align_data_len(indices_offset + indices_size)];

                unsafe {
                    data[..vertices_size].copy_from_slice(std::slice::from_raw_parts(
                        &vertices[0] as *const _ as *const _,
                        vertices_size,
                    ));

                    data[indices_offset..indices_offset + indices_size].copy_from_slice(
                        std::slice::from_raw_parts(
                            &indices[0] as *const _ as *const _,
                            indices_size,
                        ),
                    );
                }
            }

            IndexType::U32 => {
                let indices: Vec<_> = generator
                    .indexed_polygon_iter()
                    .flat_map(|polygon| {
                        let mut indices = Vec::new();

                        polygon.emit_triangles(|triangle| {
                            indices.push(triangle.x);

                            indices.push(triangle.y);

                            indices.push(triangle.z);
                        });

                        indices
                    })
                    .map(|index| u32::try_from(index).unwrap())
                    .collect();

                index_count = u32::try_from(indices.len()).map_err(|_| OutOfMemory)?;

                let indices_size = size_of_val(&indices[..]);

                data = vec![0u8; align_data_len(indices_offset + indices_size)];

                unsafe {
                    data[..vertices_size].copy_from_slice(std::slice::from_raw_parts(
                        &vertices[0] as *const _ as *const _,
                        vertices_size,
                    ));

                    data[indices_offset..indices_offset + indices_size].copy_from_slice(
                        std::slice::from_raw_parts(
                            &indices[0] as *const _ as *const _,
                            indices_size,
                        ),
                    );
                }
            }
        }

        let buffer = Buffer::from(cx.create_buffer_static(
            BufferInfo {
                align: 63,
                size: u64::try_from(data.len()).map_err(|_| OutOfMemory)?,
                usage,
            },
            &data[..],
        )?);

        let binding = Binding {
            buffer: buffer.clone(),
            offset: 0,
            layout: V::layout(),
        };

        let indices = Indices {
            buffer,
            offset: u64::try_from(indices_offset).unwrap(),
            index_type,
        };

        Ok(Mesh {
            bindings: Arc::new([binding]),
            indices: Some(indices),
            count: index_count,
            topology: PrimitiveTopology::TriangleList,
            vertex_count,
        })
    }
}

/// Mesh transformed into specific pose.
/// Contains only bindings affected by transformation -
/// i.e. bingings that contain positions, normals and/or tangets
/// FIXME: Allow sharing pose mesh in animation groups.
#[derive(Debug)]
pub struct PoseMesh {
    bindings: Arc<[Binding]>,
}

impl PoseMesh {
    /// Create new pose-mesh for mesh
    pub fn new(mesh: &Mesh, device: &Device, bump: &Bump) -> Result<Self, OutOfMemory> {
        let mut offset = 0;
        let mut prebindings = BVec::with_capacity_in(4, bump);
        let mut usage = BufferUsage::empty();

        for binding in mesh.bindings.iter() {
            let animate = binding
                .layout
                .locations
                .iter()
                .any(|l| l.semantics.animate());

            if animate {
                prebindings.push((binding.layout.clone(), offset));
                offset += binding.buffer.info().size;
                usage |= binding.buffer.info().usage;
            }
        }

        let buffer = device.create_buffer(BufferInfo {
            align: 255,
            size: offset,
            usage,
        })?;

        let bindings = prebindings
            .into_iter()
            .map(|(layout, offset)| Binding {
                layout,
                offset,
                buffer: buffer.clone(),
            })
            .collect();

        Ok(PoseMesh { bindings })
    }

    pub fn bindings(&self) -> &[Binding] {
        &*self.bindings
    }
}

fn build_triangles_blas<'a>(
    indices: Option<&Indices>,
    binding: &Binding,
    location: &VertexLocation,
    count: u32,
    vertex_count: u32,
    encoder: &mut Encoder<'a>,
    device: &Device,
    bump: &'a Bump,
) -> Result<AccelerationStructure, OutOfMemory> {
    assert_eq!(count % 3, 0);
    let triangle_count = count / 3;

    assert_eq!(binding.layout, Position3::layout());

    let pos_range = BufferRange {
        buffer: binding.buffer.clone(),
        offset: binding.offset,
        size: u64::from(Position3::layout().stride) * u64::from(vertex_count),
    };

    let sizes = device.get_acceleration_structure_build_sizes(
        AccelerationStructureLevel::Bottom,
        AccelerationStructureBuildFlags::PREFER_FAST_TRACE,
        &[AccelerationStructureGeometryInfo::Triangles {
            max_primitive_count: triangle_count,
            index_type: indices.map(|indices| indices.index_type),
            max_vertex_count: vertex_count,
            vertex_format: location.format,
            allows_transforms: true,
        }],
    );

    let acc_buffer = device.create_buffer(BufferInfo {
        align: 255,
        size: sizes.acceleration_structure_size,
        usage: BufferUsage::ACCELERATION_STRUCTURE_STORAGE,
    })?;

    let blas = device.create_acceleration_structure(AccelerationStructureInfo {
        level: AccelerationStructureLevel::Bottom,
        region: BufferRange {
            buffer: acc_buffer,
            offset: 0,
            size: sizes.acceleration_structure_size,
        },
    })?;

    let blas_scratch = device.create_buffer(BufferInfo {
        align: 255,
        size: sizes.build_scratch_size,
        usage: BufferUsage::DEVICE_ADDRESS,
    })?;

    let blas_scratch_address = device.get_buffer_device_address(&blas_scratch).unwrap();

    let geometries = bump.alloc([AccelerationStructureGeometry::Triangles {
        flags: GeometryFlags::empty(),
        vertex_format: Format::RGB32Sfloat,
        vertex_data: pos_range,
        vertex_stride: binding.layout.stride.into(),
        vertex_count,
        first_vertex: 0,
        primitive_count: triangle_count,
        index_data: indices.map(|indices| {
            let index_range = BufferRange {
                buffer: indices.buffer.clone(),
                offset: indices.offset,
                size: u64::from(indices.index_type.size()) * u64::from(count),
            };

            match indices.index_type {
                IndexType::U16 => IndexData::U16(index_range),
                IndexType::U32 => IndexData::U32(index_range),
            }
        }),
        transform_data: None,
    }]);

    encoder.build_acceleration_structure(&[AccelerationStructureBuildGeometryInfo {
        src: None,
        dst: bump.alloc(blas.clone()),
        flags: AccelerationStructureBuildFlags::PREFER_FAST_TRACE,
        geometries,
        scratch: blas_scratch_address,
    }]);

    Ok(blas)
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
    type Fut = Ready<Result<MeshFile, Self::DecodeError>>;

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
        let graphics = builder.borrow_mut();

        let bindings: Arc<[Binding]> = decoded
            .header
            .bindings
            .iter()
            .map(|binding| -> Result<_, OutOfMemory> {
                let layout = match binding.layout {
                    arcana_mesh_file::VertexLayout::Position3 => Position3::layout(),
                    arcana_mesh_file::VertexLayout::Normal3 => Normal3::layout(),
                    arcana_mesh_file::VertexLayout::Tangent3 => Tangent3::layout(),
                    arcana_mesh_file::VertexLayout::UV => UV::layout(),
                    arcana_mesh_file::VertexLayout::Color => Color::layout(),
                    arcana_mesh_file::VertexLayout::PositionNormal3 => PositionNormal3::layout(),
                    arcana_mesh_file::VertexLayout::PositionNormal3UV => {
                        PositionNormal3UV::layout()
                    }
                    arcana_mesh_file::VertexLayout::PositionNormal3Color => {
                        PositionNormal3Color::layout()
                    }
                    arcana_mesh_file::VertexLayout::PositionNormalTangent3 => {
                        PositionNormalTangent3::layout()
                    }
                    arcana_mesh_file::VertexLayout::PositionNormalTangent3UV => {
                        PositionNormalTangent3UV::layout()
                    }
                    arcana_mesh_file::VertexLayout::PositionNormalTangent3Color => {
                        PositionNormalTangent3Color::layout()
                    }
                    arcana_mesh_file::VertexLayout::Joints => Joints::layout(),
                    arcana_mesh_file::VertexLayout::Weights => Weights::layout(),
                    arcana_mesh_file::VertexLayout::Skin => Skin::layout(),
                };

                let size = u64::from(layout.stride) * u64::from(decoded.header.vertex_count);
                let size_usize = usize::try_from(size).map_err(|_| OutOfMemory)?;

                Ok(Binding {
                    buffer: graphics
                        .create_buffer_static(
                            BufferInfo {
                                align: 255,
                                size,
                                usage: BufferUsage::VERTEX,
                            },
                            &decoded.bytes[binding.offset..][..size_usize],
                        )?
                        .into(),
                    offset: 0,
                    layout,
                })
            })
            .collect::<Result<_, _>>()?;

        let mut count = decoded.header.vertex_count;

        let indices = decoded
            .header
            .indices
            .as_ref()
            .map(|indices| -> Result<_, OutOfMemory> {
                count = indices.count;

                let stride = match indices.index_type {
                    arcana_mesh_file::IndexType::U16 => 2,
                    arcana_mesh_file::IndexType::U32 => 4,
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
                            &decoded.bytes[indices.offset..][..size_usize],
                        )?
                        .into(),
                    offset: 0,
                    index_type: match indices.index_type {
                        arcana_mesh_file::IndexType::U16 => IndexType::U16,
                        arcana_mesh_file::IndexType::U32 => IndexType::U32,
                    },
                })
            })
            .transpose()?;

        Ok(Mesh {
            bindings,
            indices,
            topology: match decoded.header.topology {
                arcana_mesh_file::PrimitiveTopology::PointList => PrimitiveTopology::PointList,
                arcana_mesh_file::PrimitiveTopology::LineList => PrimitiveTopology::LineList,
                arcana_mesh_file::PrimitiveTopology::LineStrip => PrimitiveTopology::LineStrip,
                arcana_mesh_file::PrimitiveTopology::TriangleList => {
                    PrimitiveTopology::TriangleList
                }
                arcana_mesh_file::PrimitiveTopology::TriangleStrip => {
                    PrimitiveTopology::TriangleStrip
                }
                arcana_mesh_file::PrimitiveTopology::TriangleFan => PrimitiveTopology::TriangleFan,
            },
            count,
            vertex_count: decoded.header.vertex_count,
        })
    }
}
