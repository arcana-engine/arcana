use std::{borrow::Cow, fmt::Debug, mem::size_of};

use bytemuck::{Pod, Zeroable};
use sierra::{Format, VertexInputAttribute, VertexInputBinding, VertexInputRate};

#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Semantics {
    Position2,
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
    pub fn vector(&self) -> bool {
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

/// Trait for single vertex attribute.
pub trait VertexAttribute: Debug + Default + PartialEq + Pod {
    const FORMAT: Format;
    const SEMANTICS: Semantics;
}

/// Trait for vertex layouts.
pub trait VertexType: Debug + Default + PartialEq + Pod {
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

impl<T> VertexType for T
where
    T: VertexAttribute,
{
    const LOCATIONS: &'static [VertexLocation] = &[VertexLocation {
        format: T::FORMAT,
        offset: 0,
        semantics: T::SEMANTICS,
    }];
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

define_vertex_attribute! {
    /// Attribute for vertex position in 2d world.
    pub struct Position2 as (Semantics::Position2) (pub [f32; 2]);

    /// Attribute for vertex position in 3d world.
    pub struct Position3 as (Semantics::Position3) (pub [f32; 3]);

    /// Attribute for vertex normal in 3d world.
    pub struct Normal3 as (Semantics::Normal3) (pub [f32; 3]);

    /// Attribute for vertex tanggent in 3d world.
    pub struct Tangent3 as (Semantics::Tangent3) (pub [f32; 4]);

    /// Attribute for texture coordinates.
    pub struct UV as (Semantics::UV) (pub [f32; 2]);

    pub struct Joints as (Semantics::Joints) (pub [u32; 4]);

    pub struct Weights as (Semantics::Weights) (pub [f32; 4]);
}

impl VertexAttribute for palette::rgb::Srgb<u8> {
    const FORMAT: Format = Format::RGB8Srgb;
    const SEMANTICS: Semantics = Semantics::Color;
}

impl VertexAttribute for palette::rgb::Srgba<u8> {
    const FORMAT: Format = Format::RGBA8Srgb;
    const SEMANTICS: Semantics = Semantics::Color;
}

impl VertexAttribute for palette::rgb::LinSrgb<u8> {
    const FORMAT: Format = Format::RGB8Unorm;
    const SEMANTICS: Semantics = Semantics::Color;
}

impl VertexAttribute for palette::rgb::LinSrgba<u8> {
    const FORMAT: Format = Format::RGBA8Unorm;
    const SEMANTICS: Semantics = Semantics::Color;
}

impl VertexAttribute for palette::rgb::LinSrgb<u16> {
    const FORMAT: Format = Format::RGB16Unorm;
    const SEMANTICS: Semantics = Semantics::Color;
}

impl VertexAttribute for palette::rgb::LinSrgba<u16> {
    const FORMAT: Format = Format::RGBA16Unorm;
    const SEMANTICS: Semantics = Semantics::Color;
}

impl VertexAttribute for palette::rgb::LinSrgb<f32> {
    const FORMAT: Format = Format::RGB32Sfloat;
    const SEMANTICS: Semantics = Semantics::Color;
}

impl VertexAttribute for palette::rgb::LinSrgba<f32> {
    const FORMAT: Format = Format::RGBA32Sfloat;
    const SEMANTICS: Semantics = Semantics::Color;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct V2<A, B>(pub A, pub B);

unsafe impl<A: Zeroable, B: Zeroable> Zeroable for V2<A, B> {}
unsafe impl<A: Pod, B: Pod> Pod for V2<A, B> {}

impl<A, B> VertexType for V2<A, B>
where
    A: VertexAttribute,
    B: VertexAttribute,
{
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: A::FORMAT,
            offset: 0,
            semantics: A::SEMANTICS,
        },
        VertexLocation {
            format: B::FORMAT,
            offset: size_of::<A>() as u32,
            semantics: B::SEMANTICS,
        },
    ];
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct V3<A, B, C>(pub A, pub B, pub C);

unsafe impl<A: Zeroable, B: Zeroable, C: Zeroable> Zeroable for V3<A, B, C> {}
unsafe impl<A: Pod, B: Pod, C: Pod> Pod for V3<A, B, C> {}

impl<A, B, C> VertexType for V3<A, B, C>
where
    A: VertexAttribute,
    B: VertexAttribute,
    C: VertexAttribute,
{
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: A::FORMAT,
            offset: 0,
            semantics: A::SEMANTICS,
        },
        VertexLocation {
            format: B::FORMAT,
            offset: size_of::<A>() as u32,
            semantics: B::SEMANTICS,
        },
        VertexLocation {
            format: C::FORMAT,
            offset: size_of::<A>() as u32 + size_of::<B>() as u32,
            semantics: C::SEMANTICS,
        },
    ];
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct V4<A, B, C, D>(pub A, pub B, pub C, pub D);

unsafe impl<A: Zeroable, B: Zeroable, C: Zeroable, D: Zeroable> Zeroable for V4<A, B, C, D> {}
unsafe impl<A: Pod, B: Pod, C: Pod, D: Pod> Pod for V4<A, B, C, D> {}

impl<A, B, C, D> VertexType for V4<A, B, C, D>
where
    A: VertexAttribute,
    B: VertexAttribute,
    C: VertexAttribute,
    D: VertexAttribute,
{
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: A::FORMAT,
            offset: 0,
            semantics: A::SEMANTICS,
        },
        VertexLocation {
            format: B::FORMAT,
            offset: size_of::<A>() as u32,
            semantics: B::SEMANTICS,
        },
        VertexLocation {
            format: C::FORMAT,
            offset: size_of::<A>() as u32 + size_of::<B>() as u32,
            semantics: C::SEMANTICS,
        },
        VertexLocation {
            format: D::FORMAT,
            offset: size_of::<A>() as u32 + size_of::<B>() as u32 + size_of::<C>() as u32,
            semantics: D::SEMANTICS,
        },
    ];
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}

pub type Position2UV = V2<Position2, UV>;
pub type Position3UV = V2<Position3, UV>;
pub type PositionNormal3 = V2<Position3, Normal3>;
pub type PositionNormalTangent3 = V3<Position3, Normal3, Tangent3>;
pub type PositionNormal3UV = V3<Position3, Normal3, UV>;
pub type PositionNormalTangent3UV = V4<Position3, Normal3, Tangent3, UV>;
pub type Skin = V2<Joints, Weights>;

/// Attribute for instance 2d transformation.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Transformation2(pub [[f32; 3]; 3]);

impl Default for Transformation2 {
    fn default() -> Self {
        Transformation2([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    }
}

unsafe impl Zeroable for Transformation2 {}
unsafe impl Pod for Transformation2 {}

impl VertexType for Transformation2 {
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: size_of::<[[f32; 4]; 0]>() as u32,
            semantics: Semantics::Transform0,
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: size_of::<[[f32; 4]; 1]>() as u32,
            semantics: Semantics::Transform1,
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: size_of::<[[f32; 4]; 2]>() as u32,
            semantics: Semantics::Transform2,
        },
    ];
    const RATE: VertexInputRate = VertexInputRate::Instance;
}

/// Attribute for instance 3d transformation.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Transformation3(pub [[f32; 4]; 4]);

unsafe impl Zeroable for Transformation3 {}
unsafe impl Pod for Transformation3 {}

impl Default for Transformation3 {
    fn default() -> Self {
        Transformation3([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

impl VertexType for Transformation3 {
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

    impl From<Vertex> for V2<Position3, Normal3> {
        fn from(v: Vertex) -> Self {
            V2(Position3::from(v), Normal3::from(v))
        }
    }
}
