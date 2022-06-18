use std::{
    fmt::{self, Debug, Display},
    ops::RangeInclusive,
};

use bytemuck::{Pod, Zeroable};
use na::Scalar;
use num_traits::{Num, One, Zero};

#[cfg(feature = "graphics")]
use sierra::Format;

#[cfg(feature = "graphics")]
use crate::graphics::{Semantics, VertexAttribute};

#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Rect<T = f32> {
    pub left: T,
    pub right: T,
    pub bottom: T,
    pub top: T,
}

#[cfg(feature = "graphics")]
impl VertexAttribute for Rect<f32> {
    const FORMAT: Format = Format::RGBA32Sfloat;
    const SEMANTICS: Semantics = Semantics::new("Rect");
}

#[cfg(feature = "graphics")]
impl VertexAttribute for Rect<u32> {
    const FORMAT: Format = Format::RGBA32Uint;
    const SEMANTICS: Semantics = Semantics::new("Rect");
}

#[cfg(feature = "graphics")]
impl VertexAttribute for Rect<i32> {
    const FORMAT: Format = Format::RGBA32Sint;
    const SEMANTICS: Semantics = Semantics::new("Rect");
}

impl Rect {
    pub const ONE_QUAD: Rect = Rect {
        left: 0.0,
        right: 1.0,
        top: 1.0,
        bottom: 0.0,
    };
}

/// # Safety
///
/// This impl is safe because all `Rect` fields have type `T` and there can't be any padding.
unsafe impl<T> Zeroable for Rect<T> where T: Zeroable {}

/// # Safety
///
/// This impl is safe because all `Rect` fields have type `T` and there can't be any padding.
unsafe impl<T> Pod for Rect<T> where T: Pod {}

impl<T> Display for Rect<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Rect {{")?;
        writeln!(f, "\tleft: {}", self.left)?;
        writeln!(f, "\tright: {}", self.right)?;
        writeln!(f, "\ttop: {}", self.top)?;
        writeln!(f, "\tbottom: {}", self.bottom)?;
        writeln!(f, "}}")
    }
}

impl<T> Debug for Rect<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Rect {{")?;
        writeln!(f, "\tleft: {:?}", self.left)?;
        writeln!(f, "\tright: {:?}", self.right)?;
        writeln!(f, "\ttop: {:?}", self.top)?;
        writeln!(f, "\tbottom: {:?}", self.bottom)?;
        writeln!(f, "}}")
    }
}

impl<T> Default for Rect<T>
where
    T: Zero + One,
{
    fn default() -> Self {
        Rect {
            left: T::zero(),
            right: T::one(),
            top: T::one(),
            bottom: T::zero(),
        }
    }
}

impl<T> Rect<T> {
    pub fn flip_vertical(self) -> Self {
        Rect {
            left: self.left,
            right: self.right,
            top: self.bottom,
            bottom: self.top,
        }
    }

    pub fn flip_horizontal(self) -> Self {
        Rect {
            left: self.right,
            right: self.left,
            top: self.top,
            bottom: self.bottom,
        }
    }
}

pub struct RectIter<T> {
    line: RangeInclusive<T>,
    start: T,
    vertical: RangeInclusive<T>,
}

impl<T> Iterator for RectIter<T>
where
    RangeInclusive<T>: Iterator<Item = T>,
    T: Scalar,
{
    type Item = na::Point2<T>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (line_lower, line_upper) = self.line.size_hint();

        let (horizontal_lower, horizontal_upper) =
            (self.start.clone()..=self.line.end().clone()).size_hint();

        let (vertical_lower, vertical_upper) = self.vertical.size_hint();

        let lower = line_lower.saturating_add(horizontal_lower.saturating_mul(vertical_lower));

        let upper = match (line_upper, horizontal_upper, vertical_upper) {
            (Some(line_upper), Some(horizontal_upper), Some(vertical_upper)) => horizontal_upper
                .checked_mul(vertical_upper)
                .and_then(|hv| line_upper.checked_add(hv)),
            _ => None,
        };

        (lower, upper)
    }

    fn next(&mut self) -> Option<na::Point2<T>> {
        match self.line.next() {
            None => {
                self.vertical.next()?;
                self.line = self.start.clone()..=self.line.end().clone();

                match self.line.next() {
                    None => {
                        self.vertical.by_ref().count();
                        None
                    }
                    Some(x) => Some(na::Point2::new(x, self.vertical.start().clone())),
                }
            }
            Some(x) => Some(na::Point2::new(x, self.vertical.start().clone())),
        }
    }

    fn fold<B, F>(mut self, init: B, mut f: F) -> B
    where
        F: FnMut(B, na::Point2<T>) -> B,
    {
        let mut acc = init;
        for x in self.line.by_ref() {
            acc = f(acc, na::Point2::new(x, self.vertical.start().clone()));
        }
        let line = self.start.clone()..=self.line.end().clone();
        for y in self.vertical.by_ref() {
            for x in line.clone() {
                acc = f(acc, na::Point2::new(x, y.clone()));
            }
        }
        acc
    }
}

impl<T> Rect<T>
where
    T: Clone,
{
    pub fn iter(&self) -> RectIter<T> {
        RectIter {
            line: self.left.clone()..=self.right.clone(),
            start: self.left.clone(),
            vertical: self.bottom.clone()..=self.top.clone(),
        }
    }
}

impl<T> Rect<T>
where
    T: Scalar + PartialOrd,
{
    pub fn contains(&self, point: &na::Point2<T>) -> bool {
        self.left <= point.x
            && self.right >= point.x
            && self.bottom <= point.y
            && self.top >= point.y
    }

    pub fn overlap(&self, other: &Rect<T>) -> Rect<T> {
        Rect {
            left: if self.left < other.left {
                self.left.clone()
            } else {
                other.left.clone()
            },
            right: if self.right >= other.right {
                self.right.clone()
            } else {
                other.right.clone()
            },
            bottom: if self.bottom < other.bottom {
                self.bottom.clone()
            } else {
                other.bottom.clone()
            },
            top: if self.top >= other.top {
                self.top.clone()
            } else {
                other.top.clone()
            },
        }
    }
}

impl<T> Rect<T>
where
    T: na::Scalar,
{
    pub fn top_left(&self) -> na::Point2<T> {
        na::Point2::new(self.left.clone(), self.top.clone())
    }

    pub fn bottom_left(&self) -> na::Point2<T> {
        na::Point2::new(self.left.clone(), self.bottom.clone())
    }

    pub fn top_right(&self) -> na::Point2<T> {
        na::Point2::new(self.right.clone(), self.top.clone())
    }

    pub fn bottom_right(&self) -> na::Point2<T> {
        na::Point2::new(self.right.clone(), self.bottom.clone())
    }
}

impl<T> Rect<T>
where
    T: Num + Copy,
{
    pub fn width(&self) -> T {
        self.right - self.left
    }

    pub fn height(&self) -> T {
        self.top - self.bottom
    }

    pub fn relative_to(&self, rhs: &Rect<T>) -> Rect<T> {
        let x = |x| (x - rhs.left) / (rhs.right - rhs.left);
        let y = |y| (y - rhs.bottom) / (rhs.top - rhs.bottom);

        Rect {
            left: x(self.left),
            right: x(self.right),
            top: y(self.top),
            bottom: y(self.bottom),
        }
    }

    pub fn from_relative_to(&self, rhs: &Rect<T>) -> Rect<T> {
        let x = |x| x * (rhs.right - rhs.left) + rhs.left;
        let y = |y| y * (rhs.top - rhs.bottom) + rhs.bottom;

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
    struct Lrtb<T> {
        #[serde(alias = "l")]
        left: T,

        #[serde(alias = "r")]
        right: T,

        #[serde(alias = "t")]
        top: T,

        #[serde(alias = "b")]
        bottom: T,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename = "XYWH")]
    struct Xywh<T> {
        x: T,
        y: T,

        #[serde(alias = "width")]
        w: T,

        #[serde(alias = "height")]
        h: T,
    }

    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum AnyRect<T> {
        Lrtb(Lrtb<T>),
        Xywh(Xywh<T>),
    }

    impl<'de, T> Deserialize<'de> for Rect<T>
    where
        T: Deserialize<'de> + Num + Copy,
    {
        fn deserialize<D>(deserializer: D) -> Result<Rect<T>, D::Error>
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
                    top: xywh.y + xywh.h,
                    bottom: xywh.y,
                },
            };
            Ok(rect)
        }
    }

    impl<T> Serialize for Rect<T>
    where
        T: Serialize,
    {
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
