use std::fmt::{self, Debug, Display};

use bytemuck::{Pod, Zeroable};

#[cfg(feature = "graphics")]
use sierra::Format;

#[cfg(feature = "graphics")]
use crate::graphics::{Semantics, VertexAttribute};

#[derive(Clone, Copy, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct Rect {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Rect {{")?;
        writeln!(f, "\tleft: {}", self.left)?;
        writeln!(f, "\tright: {}", self.right)?;
        writeln!(f, "\ttop: {}", self.top)?;
        writeln!(f, "\tbottom: {}", self.bottom)?;
        writeln!(f, "}}")
    }
}

impl Debug for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

#[cfg(feature = "graphics")]
impl VertexAttribute for Rect {
    const FORMAT: Format = Format::RGBA32Sfloat;
    const SEMANTICS: Semantics = Semantics::new("Rect");
}

impl Default for Rect {
    fn default() -> Self {
        Rect {
            left: 0.0,
            right: 1.0,
            top: 0.0,
            bottom: 1.0,
        }
    }
}

impl Rect {
    pub const ONE_QUAD: Rect = Rect {
        left: 0.0,
        right: 1.0,
        top: 0.0,
        bottom: 1.0,
    };

    pub fn relative_to(&self, rhs: &Rect) -> Rect {
        let x = |x| (x - rhs.left) / (rhs.right - rhs.left);
        let y = |y| (y - rhs.top) / (rhs.bottom - rhs.top);

        Rect {
            left: x(self.left),
            right: x(self.right),
            top: y(self.top),
            bottom: y(self.bottom),
        }
    }

    pub fn from_relative_to(&self, rhs: &Rect) -> Rect {
        let x = |x| x * (rhs.right - rhs.left) + rhs.left;
        let y = |y| y * (rhs.bottom - rhs.top) + rhs.top;

        Rect {
            left: x(self.left),
            right: x(self.right),
            top: y(self.top),
            bottom: y(self.bottom),
        }
    }
}

mod serde_impls {
    use {
        super::*,
        serde::{de::*, ser::*},
    };

    #[derive(serde::Deserialize)]
    #[serde(rename = "LRTB")]
    struct Lrtb {
        #[serde(alias = "l")]
        left: f32,

        #[serde(alias = "r")]
        right: f32,

        #[serde(alias = "t")]
        top: f32,

        #[serde(alias = "b")]
        bottom: f32,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename = "XYWH")]
    struct Xywh {
        x: f32,
        y: f32,

        #[serde(alias = "width")]
        w: f32,

        #[serde(alias = "height")]
        h: f32,
    }

    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum AnyRect {
        Lrtb(Lrtb),
        Xywh(Xywh),
    }

    impl<'de> Deserialize<'de> for Rect {
        fn deserialize<D>(deserializer: D) -> Result<Rect, D::Error>
        where
            D: Deserializer<'de>,
        {
            let rect = match AnyRect::deserialize(deserializer)? {
                AnyRect::Lrtb(lrtb) => Rect {
                    left: lrtb.left,
                    right: lrtb.right,
                    top: lrtb.top,
                    bottom: lrtb.bottom,
                },
                AnyRect::Xywh(xywh) => Rect {
                    left: xywh.x,
                    right: xywh.x + xywh.w,
                    top: xywh.y,
                    bottom: xywh.y + xywh.h,
                },
            };
            Ok(rect)
        }
    }

    impl Serialize for Rect {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut serializer = serializer.serialize_struct("Rect", 4)?;
            serializer.serialize_field("l", &self.left)?;
            serializer.serialize_field("r", &self.right)?;
            serializer.serialize_field("t", &self.top)?;
            serializer.serialize_field("b", &self.bottom)?;
            serializer.end()
        }
    }
}
