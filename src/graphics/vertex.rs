use {
    bytemuck::{Pod, Zeroable},
    std::{borrow::Cow, mem::size_of},
};

pub use sierra::{Format, VertexInputAttribute, VertexInputBinding, VertexInputRate};

#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Semantics {
    Position3,
    Normal3,
    Tangent3,
    UV,
    Color,
    Joints,
    Weights,
    Transform0,
    Transform1,
    Transform2,
    Transform3,
    Custom(Cow<'static, str>),
}

impl Semantics {
    pub fn animate(&self) -> bool {
        matches!(
            self,
            Semantics::Position3 | Semantics::Normal3 | Semantics::Tangent3
        )
    }
}

/// Describes single vertex location.
#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VertexLocation {
    /// Specifies how data is interpreted for attributes.
    /// Attribute component types in vertex shader must match base type of the
    /// format.
    pub format: Format,

    /// Offset of data in vertex buffer element.
    pub offset: u32,

    /// Vertex attribute semantics.
    pub semantics: Semantics,
}

/// Describes layout of vertex buffer element.
#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VertexLayout {
    pub locations: Cow<'static, [VertexLocation]>,
    pub stride: u32,
    pub rate: VertexInputRate,
}

/// Trait for vertex layouts.
pub trait VertexType: Pod {
    const NAME: &'static str;
    const LOCATIONS: &'static [VertexLocation];
    const RATE: VertexInputRate;

    /// Get layout of this vertex type.
    fn layout() -> VertexLayout
    where
        Self: Sized,
    {
        VertexLayout {
            locations: Cow::Borrowed(Self::LOCATIONS),
            stride: size_of::<Self>() as u32,
            rate: Self::RATE,
        }
    }
}

/// Attribute for vertex position in 3d world.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Position3(pub [f32; 3]);

unsafe impl Zeroable for Position3 {}
unsafe impl Pod for Position3 {}

impl VertexType for Position3 {
    const LOCATIONS: &'static [VertexLocation] = &[VertexLocation {
        format: Format::RGB32Sfloat,
        offset: 0,
        semantics: Semantics::Position3,
    }];
    const NAME: &'static str = "Position3";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

/// Attribute for vertex normal in 3d world.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Normal3(pub [f32; 3]);

unsafe impl Zeroable for Normal3 {}
unsafe impl Pod for Normal3 {}

impl VertexType for Normal3 {
    const LOCATIONS: &'static [VertexLocation] = &[VertexLocation {
        format: Format::RGB32Sfloat,
        offset: 0,
        semantics: Semantics::Normal3,
    }];
    const NAME: &'static str = "Normal3";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

/// Attribute for vertex position in 3d world.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Tangent3(pub [f32; 4]);

unsafe impl Zeroable for Tangent3 {}
unsafe impl Pod for Tangent3 {}

impl VertexType for Tangent3 {
    const LOCATIONS: &'static [VertexLocation] = &[VertexLocation {
        format: Format::RGBA32Sfloat,
        offset: 0,
        semantics: Semantics::Tangent3,
    }];
    const NAME: &'static str = "Tangent3";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

/// Attribute for vertex color.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Color(pub [f32; 4]);

unsafe impl Zeroable for Color {}
unsafe impl Pod for Color {}

impl VertexType for Color {
    const LOCATIONS: &'static [VertexLocation] = &[VertexLocation {
        format: Format::RGBA32Sfloat,
        offset: 0,
        semantics: Semantics::Color,
    }];
    const NAME: &'static str = "Color";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

/// Attribute for texture coordinates.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct UV(pub [f32; 2]);

unsafe impl Zeroable for UV {}
unsafe impl Pod for UV {}

impl VertexType for UV {
    const LOCATIONS: &'static [VertexLocation] = &[VertexLocation {
        format: Format::RG32Sfloat,
        offset: 0,
        semantics: Semantics::UV,
    }];
    const NAME: &'static str = "UV";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

/// Attribute for texture coordinates.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Joints(pub [u32; 4]);

unsafe impl Zeroable for Joints {}
unsafe impl Pod for Joints {}

impl VertexType for Joints {
    const LOCATIONS: &'static [VertexLocation] = &[VertexLocation {
        format: Format::RGBA32Uint,
        offset: 0,
        semantics: Semantics::Joints,
    }];
    const NAME: &'static str = "Joints";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

/// Attribute for texture coordinates.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Weights(pub [f32; 4]);

unsafe impl Zeroable for Weights {}
unsafe impl Pod for Weights {}

impl VertexType for Weights {
    const LOCATIONS: &'static [VertexLocation] = &[VertexLocation {
        format: Format::RGBA32Sfloat,
        offset: 0,
        semantics: Semantics::Weights,
    }];
    const NAME: &'static str = "Weights";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct Position3UV {
    pub position: Position3,
    pub uv: UV,
}

unsafe impl Zeroable for Position3UV {}
unsafe impl Pod for Position3UV {}

impl VertexType for Position3UV {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 0,
            semantics: Semantics::Position3,
        },
        VertexLocation {
            format: Format::RG32Sfloat,
            offset: size_of::<Position3>() as u32,
            semantics: Semantics::Position3,
        },
    ];
    const NAME: &'static str = "Position3UV";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct Position3Color {
    pub position: Position3,
    pub color: Color,
}

unsafe impl Zeroable for Position3Color {}
unsafe impl Pod for Position3Color {}

impl VertexType for Position3Color {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 0,
            semantics: Semantics::Position3,
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<Position3>() as u32,
            semantics: Semantics::Color,
        },
    ];
    const NAME: &'static str = "Position3Color";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct PositionNormal3 {
    pub position: Position3,
    pub normal: Normal3,
}

unsafe impl Zeroable for PositionNormal3 {}
unsafe impl Pod for PositionNormal3 {}

impl VertexType for PositionNormal3 {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 0,
            semantics: Semantics::Position3,
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: size_of::<Position3>() as u32,
            semantics: Semantics::Normal3,
        },
    ];
    const NAME: &'static str = "PositionNormal3";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct PositionNormalTangent3 {
    pub position: Position3,
    pub normal: Normal3,
    pub tangent: Tangent3,
}

unsafe impl Zeroable for PositionNormalTangent3 {}
unsafe impl Pod for PositionNormalTangent3 {}

impl VertexType for PositionNormalTangent3 {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 0,
            semantics: Semantics::Position3,
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: size_of::<Position3>() as u32,
            semantics: Semantics::Normal3,
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<Position3>() as u32 + size_of::<Normal3>() as u32,
            semantics: Semantics::Tangent3,
        },
    ];
    const NAME: &'static str = "PositionNormalTangent3";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct PositionNormal3UV {
    pub position: Position3,
    pub normal: Normal3,
    pub uv: UV,
}

unsafe impl Zeroable for PositionNormal3UV {}
unsafe impl Pod for PositionNormal3UV {}

impl VertexType for PositionNormal3UV {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 0,
            semantics: Semantics::Position3,
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: size_of::<Position3>() as u32,
            semantics: Semantics::Normal3,
        },
        VertexLocation {
            format: Format::RG32Sfloat,
            offset: size_of::<Position3>() as u32 + size_of::<Normal3>() as u32,
            semantics: Semantics::UV,
        },
    ];
    const NAME: &'static str = "PositionNormal3UV";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct PositionNormalTangent3UV {
    pub position: Position3,
    pub normal: Normal3,
    pub tangent: Tangent3,
    pub uv: UV,
}

unsafe impl Zeroable for PositionNormalTangent3UV {}
unsafe impl Pod for PositionNormalTangent3UV {}

impl VertexType for PositionNormalTangent3UV {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 0,
            semantics: Semantics::Position3,
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: size_of::<Position3>() as u32,
            semantics: Semantics::Normal3,
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<Position3>() as u32 + size_of::<Normal3>() as u32,
            semantics: Semantics::Tangent3,
        },
        VertexLocation {
            format: Format::RG32Sfloat,
            offset: size_of::<Position3>() as u32
                + size_of::<Normal3>() as u32
                + size_of::<Tangent3>() as u32,
            semantics: Semantics::UV,
        },
    ];
    const NAME: &'static str = "PositionNormalTangent3UV";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct PositionNormal3Color {
    pub position: Position3,
    pub normal: Normal3,
    pub color: Color,
}

unsafe impl Zeroable for PositionNormal3Color {}
unsafe impl Pod for PositionNormal3Color {}

impl VertexType for PositionNormal3Color {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 0,
            semantics: Semantics::Position3,
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: size_of::<Position3>() as u32,
            semantics: Semantics::Normal3,
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<Position3>() as u32 + size_of::<Normal3>() as u32,
            semantics: Semantics::Color,
        },
    ];
    const NAME: &'static str = "PositionNormal3Color";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct PositionNormalTangent3Color {
    pub position: Position3,
    pub normal: Normal3,
    pub tangent: Tangent3,
    pub color: Color,
}

unsafe impl Zeroable for PositionNormalTangent3Color {}
unsafe impl Pod for PositionNormalTangent3Color {}

impl VertexType for PositionNormalTangent3Color {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 0,
            semantics: Semantics::Position3,
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: size_of::<Position3>() as u32,
            semantics: Semantics::Normal3,
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<Position3>() as u32 + size_of::<Normal3>() as u32,
            semantics: Semantics::Tangent3,
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<Position3>() as u32
                + size_of::<Normal3>() as u32
                + size_of::<Tangent3>() as u32,
            semantics: Semantics::Color,
        },
    ];
    const NAME: &'static str = "PositionNormalTangent3Color";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct Skin {
    pub joints: Joints,
    pub weights: Weights,
}

unsafe impl Zeroable for Skin {}
unsafe impl Pod for Skin {}

impl VertexType for Skin {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGBA32Uint,
            offset: 0,
            semantics: Semantics::Joints,
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<Joints>() as u32,
            semantics: Semantics::Weights,
        },
    ];
    const NAME: &'static str = "Skin";
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

/// Attribute for instance 3d transformation.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Transformation3d([[f32; 4]; 4]);

unsafe impl Zeroable for Transformation3d {}
unsafe impl Pod for Transformation3d {}

impl VertexType for Transformation3d {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<[[f32; 4]; 0]>() as u32,
            semantics: Semantics::Transform0,
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<[[f32; 4]; 1]>() as u32,
            semantics: Semantics::Transform1,
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<[[f32; 4]; 2]>() as u32,
            semantics: Semantics::Transform2,
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: size_of::<[[f32; 4]; 3]>() as u32,
            semantics: Semantics::Transform3,
        },
    ];
    const NAME: &'static str = "Transformation3d";
    const RATE: VertexInputRate = VertexInputRate::Instance;
}

pub fn vertex_layouts_for_pipeline(
    layouts: &[VertexLayout],
) -> (Vec<VertexInputBinding>, Vec<VertexInputAttribute>) {
    let mut next_location = 0;

    let mut locations = Vec::new();

    let bindings = layouts
        .iter()
        .enumerate()
        .map(|(binding, layout)| {
            locations.extend(layout.locations.iter().map(|layout| {
                next_location += 1;

                VertexInputAttribute {
                    location: next_location - 1,
                    format: layout.format,
                    offset: layout.offset,
                    binding: binding as u32,
                }
            }));

            VertexInputBinding {
                stride: layout.stride,
                rate: layout.rate,
            }
        })
        .collect();

    (bindings, locations)
}

#[cfg(feature = "genmesh")]
mod gm {
    use super::*;
    use genmesh::Vertex;

    impl From<Vertex> for Position3 {
        fn from(v: Vertex) -> Self {
            Position3([v.pos.x, v.pos.y, v.pos.z])
        }
    }

    impl From<Vertex> for Normal3 {
        fn from(v: Vertex) -> Self {
            Normal3([v.normal.x, v.normal.y, v.normal.z])
        }
    }

    impl From<Vertex> for PositionNormal3 {
        fn from(v: Vertex) -> Self {
            PositionNormal3 {
                position: v.into(),
                normal: v.into(),
            }
        }
    }
}
