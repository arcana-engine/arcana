#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum IndexType {
    U16,
    U32,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
    TriangleFan,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum VertexLayout {
    Position3,
    Normal3,
    Tangent3,
    UV,
    PositionNormal3,
    PositionNormal3UV,
    PositionNormalTangent3,
    PositionNormalTangent3UV,
    Joints,
    Weights,
    Skin,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BindingFileHeader {
    pub offset: usize,
    pub layout: VertexLayout,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IndicesFileHeader {
    pub offset: usize,
    pub count: u32,
    pub index_type: IndexType,
}

/// Header for internal mesh file format.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MeshFileHeader {
    pub magic: u32,
    pub vertex_count: u32,
    pub bindings: Vec<BindingFileHeader>,
    pub indices: Option<IndicesFileHeader>,
    pub topology: PrimitiveTopology,
}

pub struct MeshFile {
    pub header: MeshFileHeader,
    pub bytes: Box<[u8]>,
}

impl MeshFileHeader {
    pub const MAGIC: u32 = u32::from_le_bytes(*b"msha");
}
