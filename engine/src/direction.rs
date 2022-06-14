use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_4, PI},
    mem::replace,
    ops::Neg,
};

use na::Scalar;
use num_traits::{One, Zero};

/// One of the four cardinal directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Cardinal {
    /// North direction
    /// Co-directed with Y axis
    #[serde(rename = "N", alias = "n", alias = "north", alias = "North")]
    North, // +y

    /// East direction
    /// Co-directed with X axis
    #[serde(rename = "E", alias = "e", alias = "east", alias = "East")]
    East, // +x

    /// South direction
    /// Counter-directed with Y axis
    #[serde(rename = "S", alias = "s", alias = "south", alias = "South")]
    South, // -y

    /// West direction
    /// Counter-directed with X axis
    #[serde(rename = "W", alias = "w", alias = "west", alias = "West")]
    West, // -x
}

pub struct CardinalClockwise {
    next: Cardinal,
}

impl Iterator for CardinalClockwise {
    type Item = Cardinal;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }

    fn next(&mut self) -> Option<Cardinal> {
        let next = self.next.next_clockwise();
        Some(replace(&mut self.next, next))
    }
}

pub struct CardinalEnumerate {
    next: Option<Cardinal>,
}

impl Iterator for CardinalEnumerate {
    type Item = Cardinal;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    fn next(&mut self) -> Option<Cardinal> {
        let next = match self.next? {
            Cardinal::North => Some(Cardinal::East),
            Cardinal::East => Some(Cardinal::South),
            Cardinal::South => Some(Cardinal::West),
            Cardinal::West => None,
        };
        replace(&mut self.next, next)
    }
}

impl ExactSizeIterator for CardinalEnumerate {
    fn len(&self) -> usize {
        match self.next {
            Some(Cardinal::North) => 4,
            Some(Cardinal::East) => 3,
            Some(Cardinal::South) => 2,
            Some(Cardinal::West) => 1,
            None => 0,
        }
    }
}

impl Cardinal {
    pub const ARRAY: [Cardinal; 4] = [
        Cardinal::North,
        Cardinal::East,
        Cardinal::South,
        Cardinal::West,
    ];

    #[inline(always)]
    pub const fn next_clockwise(&self) -> Cardinal {
        match self {
            Cardinal::North => Cardinal::East,
            Cardinal::East => Cardinal::South,
            Cardinal::South => Cardinal::West,
            Cardinal::West => Cardinal::North,
        }
    }

    #[inline(always)]
    pub const fn iter_clockwise(&self) -> CardinalClockwise {
        CardinalClockwise { next: *self }
    }

    #[inline(always)]
    pub const fn enumerate() -> CardinalEnumerate {
        CardinalEnumerate {
            next: Some(Cardinal::North),
        }
    }

    #[inline(always)]
    pub fn angle(&self) -> f32 {
        match self {
            Cardinal::North => 0.0,
            Cardinal::East => FRAC_PI_2,
            Cardinal::South => PI,
            Cardinal::West => PI + FRAC_PI_2,
        }
    }

    #[inline(always)]
    pub fn vector<T>(&self) -> na::Vector2<T>
    where
        T: Zero + One + Neg<Output = T> + Scalar,
    {
        match self {
            Cardinal::North => na::Vector2::y(),
            Cardinal::East => na::Vector2::x(),
            Cardinal::South => -na::Vector2::y(),
            Cardinal::West => -na::Vector2::x(),
        }
    }

    #[inline(always)]
    pub fn from_vector<T>(vector: na::Vector2<T>) -> Option<Self>
    where
        T: Zero + Neg<Output = T> + PartialOrd + Scalar,
    {
        if vector.x == T::zero() && vector.y == T::zero() {
            None
        } else {
            if vector.x < T::zero() {
                if vector.y < T::zero() {
                    if vector.x > vector.y {
                        Some(Cardinal::South)
                    } else {
                        Some(Cardinal::West)
                    }
                } else {
                    if -vector.x.clone() < vector.y {
                        Some(Cardinal::North)
                    } else {
                        Some(Cardinal::West)
                    }
                }
            } else {
                if vector.y < T::zero() {
                    if vector.x < -vector.y.clone() {
                        Some(Cardinal::South)
                    } else {
                        Some(Cardinal::East)
                    }
                } else {
                    if vector.x < vector.y {
                        Some(Cardinal::North)
                    } else {
                        Some(Cardinal::East)
                    }
                }
            }
        }
    }
}

/// One of the four ordinal directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Ordinal {
    #[serde(rename = "NE", alias = "ne", alias = "north-east", alias = "NorthEast")]
    NorthEast,
    #[serde(rename = "SE", alias = "se", alias = "south-east", alias = "SouthEast")]
    SouthEast,
    #[serde(rename = "SW", alias = "sw", alias = "south-west", alias = "SouthWest")]
    SouthWest,
    #[serde(rename = "NW", alias = "nw", alias = "north-west", alias = "NorthWest")]
    NorthWest,
}

impl Ordinal {
    pub const ARRAY: [Ordinal; 4] = [
        Ordinal::NorthEast,
        Ordinal::SouthEast,
        Ordinal::SouthWest,
        Ordinal::NorthWest,
    ];

    #[inline(always)]
    pub fn iter() -> impl Iterator<Item = Ordinal> {
        <[_; 4]>::into_iter(Self::ARRAY)
    }

    #[inline(always)]
    pub fn angle(&self) -> f32 {
        match self {
            Ordinal::NorthEast => FRAC_PI_4,
            Ordinal::SouthEast => FRAC_PI_2 + FRAC_PI_4,
            Ordinal::SouthWest => PI + FRAC_PI_4,
            Ordinal::NorthWest => PI + FRAC_PI_2 + FRAC_PI_4,
        }
    }
}
