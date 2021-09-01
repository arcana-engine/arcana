use std::fmt::Debug;

use bytemuck::{Pod, Zeroable};
use sierra::Format;

pub trait FormatElement: Clone + Copy + Debug + Default + PartialEq + PartialOrd + Pod {
    const FORMAT: Format;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-1", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde-1", serde(transparent))]
#[repr(transparent)]
pub struct Srgb(pub u8);

unsafe impl Zeroable for Srgb {}
unsafe impl Pod for Srgb {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-1", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde-1", serde(transparent))]
#[repr(transparent)]
pub struct Norm<T>(pub T);

unsafe impl<T: Zeroable> Zeroable for Norm<T> {}
unsafe impl<T: Pod> Pod for Norm<T> {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-1", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde-1", serde(transparent))]
#[repr(transparent)]
pub struct Scaled<T>(pub T);

unsafe impl<T: Zeroable> Zeroable for Scaled<T> {}
unsafe impl<T: Pod> Pod for Scaled<T> {}

impl FormatElement for u8 {
    const FORMAT: Format = Format::R8Uint;
}

impl FormatElement for Srgb {
    const FORMAT: Format = Format::R8Srgb;
}

impl FormatElement for Norm<u8> {
    const FORMAT: Format = Format::R8Unorm;
}

impl FormatElement for Scaled<u8> {
    const FORMAT: Format = Format::R8Uscaled;
}

impl FormatElement for u16 {
    const FORMAT: Format = Format::R16Uint;
}

impl FormatElement for Norm<u16> {
    const FORMAT: Format = Format::R16Unorm;
}

impl FormatElement for Scaled<u16> {
    const FORMAT: Format = Format::R16Uscaled;
}

impl FormatElement for u32 {
    const FORMAT: Format = Format::R32Uint;
}

impl FormatElement for u64 {
    const FORMAT: Format = Format::R64Uint;
}

impl FormatElement for i8 {
    const FORMAT: Format = Format::R8Sint;
}

impl FormatElement for Norm<i8> {
    const FORMAT: Format = Format::R8Snorm;
}

impl FormatElement for Scaled<i8> {
    const FORMAT: Format = Format::R8Sscaled;
}

impl FormatElement for i16 {
    const FORMAT: Format = Format::R16Sint;
}

impl FormatElement for Norm<i16> {
    const FORMAT: Format = Format::R16Snorm;
}

impl FormatElement for Scaled<i16> {
    const FORMAT: Format = Format::R16Sscaled;
}

impl FormatElement for i32 {
    const FORMAT: Format = Format::R32Sint;
}

impl FormatElement for i64 {
    const FORMAT: Format = Format::R64Sint;
}

impl FormatElement for f32 {
    const FORMAT: Format = Format::R32Sfloat;
}

impl FormatElement for f64 {
    const FORMAT: Format = Format::R64Sfloat;
}

impl FormatElement for [u8; 1] {
    const FORMAT: Format = Format::R8Uint;
}

impl FormatElement for [Srgb; 1] {
    const FORMAT: Format = Format::R8Srgb;
}

impl FormatElement for [Norm<u8>; 1] {
    const FORMAT: Format = Format::R8Unorm;
}

impl FormatElement for [Scaled<u8>; 1] {
    const FORMAT: Format = Format::R8Uscaled;
}

impl FormatElement for [u16; 1] {
    const FORMAT: Format = Format::R16Uint;
}

impl FormatElement for [Norm<u16>; 1] {
    const FORMAT: Format = Format::R16Unorm;
}

impl FormatElement for [Scaled<u16>; 1] {
    const FORMAT: Format = Format::R16Uscaled;
}

impl FormatElement for [u32; 1] {
    const FORMAT: Format = Format::R32Uint;
}

impl FormatElement for [u64; 1] {
    const FORMAT: Format = Format::R64Uint;
}

impl FormatElement for [i8; 1] {
    const FORMAT: Format = Format::R8Sint;
}

impl FormatElement for [Norm<i8>; 1] {
    const FORMAT: Format = Format::R8Snorm;
}

impl FormatElement for [Scaled<i8>; 1] {
    const FORMAT: Format = Format::R8Sscaled;
}

impl FormatElement for [i16; 1] {
    const FORMAT: Format = Format::R16Sint;
}

impl FormatElement for [Norm<i16>; 1] {
    const FORMAT: Format = Format::R16Snorm;
}

impl FormatElement for [Scaled<i16>; 1] {
    const FORMAT: Format = Format::R16Sscaled;
}

impl FormatElement for [i32; 1] {
    const FORMAT: Format = Format::R32Sint;
}

impl FormatElement for [i64; 1] {
    const FORMAT: Format = Format::R64Sint;
}

impl FormatElement for [f32; 1] {
    const FORMAT: Format = Format::R32Sfloat;
}

impl FormatElement for [f64; 1] {
    const FORMAT: Format = Format::R64Sfloat;
}

impl FormatElement for [u8; 2] {
    const FORMAT: Format = Format::RG8Uint;
}

impl FormatElement for [Srgb; 2] {
    const FORMAT: Format = Format::RG8Srgb;
}

impl FormatElement for [Norm<u8>; 2] {
    const FORMAT: Format = Format::RG8Unorm;
}

impl FormatElement for [Scaled<u8>; 2] {
    const FORMAT: Format = Format::RG8Uscaled;
}

impl FormatElement for [u16; 2] {
    const FORMAT: Format = Format::RG16Uint;
}

impl FormatElement for [Norm<u16>; 2] {
    const FORMAT: Format = Format::RG16Unorm;
}

impl FormatElement for [Scaled<u16>; 2] {
    const FORMAT: Format = Format::RG16Uscaled;
}

impl FormatElement for [u32; 2] {
    const FORMAT: Format = Format::RG32Uint;
}

impl FormatElement for [u64; 2] {
    const FORMAT: Format = Format::RG64Uint;
}

impl FormatElement for [i8; 2] {
    const FORMAT: Format = Format::RG8Sint;
}

impl FormatElement for [Norm<i8>; 2] {
    const FORMAT: Format = Format::RG8Snorm;
}

impl FormatElement for [Scaled<i8>; 2] {
    const FORMAT: Format = Format::RG8Sscaled;
}

impl FormatElement for [i16; 2] {
    const FORMAT: Format = Format::RG16Sint;
}

impl FormatElement for [Norm<i16>; 2] {
    const FORMAT: Format = Format::RG16Snorm;
}

impl FormatElement for [Scaled<i16>; 2] {
    const FORMAT: Format = Format::RG16Sscaled;
}

impl FormatElement for [i32; 2] {
    const FORMAT: Format = Format::RG32Sint;
}

impl FormatElement for [i64; 2] {
    const FORMAT: Format = Format::RG64Sint;
}

impl FormatElement for [f32; 2] {
    const FORMAT: Format = Format::RG32Sfloat;
}

impl FormatElement for [f64; 2] {
    const FORMAT: Format = Format::RG64Sfloat;
}

impl FormatElement for [u8; 3] {
    const FORMAT: Format = Format::RGB8Uint;
}

impl FormatElement for [Srgb; 3] {
    const FORMAT: Format = Format::RGB8Srgb;
}

impl FormatElement for [Norm<u8>; 3] {
    const FORMAT: Format = Format::RGB8Unorm;
}

impl FormatElement for [Scaled<u8>; 3] {
    const FORMAT: Format = Format::RGB8Uscaled;
}

impl FormatElement for [u16; 3] {
    const FORMAT: Format = Format::RGB16Uint;
}

impl FormatElement for [Norm<u16>; 3] {
    const FORMAT: Format = Format::RGB16Unorm;
}

impl FormatElement for [Scaled<u16>; 3] {
    const FORMAT: Format = Format::RGB16Uscaled;
}

impl FormatElement for [u32; 3] {
    const FORMAT: Format = Format::RGB32Uint;
}

impl FormatElement for [u64; 3] {
    const FORMAT: Format = Format::RGB64Uint;
}

impl FormatElement for [i8; 3] {
    const FORMAT: Format = Format::RGB8Sint;
}

impl FormatElement for [Norm<i8>; 3] {
    const FORMAT: Format = Format::RGB8Snorm;
}

impl FormatElement for [Scaled<i8>; 3] {
    const FORMAT: Format = Format::RGB8Sscaled;
}

impl FormatElement for [i16; 3] {
    const FORMAT: Format = Format::RGB16Sint;
}

impl FormatElement for [Norm<i16>; 3] {
    const FORMAT: Format = Format::RGB16Snorm;
}

impl FormatElement for [Scaled<i16>; 3] {
    const FORMAT: Format = Format::RGB16Sscaled;
}

impl FormatElement for [i32; 3] {
    const FORMAT: Format = Format::RGB32Sint;
}

impl FormatElement for [i64; 3] {
    const FORMAT: Format = Format::RGB64Sint;
}

impl FormatElement for [f32; 3] {
    const FORMAT: Format = Format::RGB32Sfloat;
}

impl FormatElement for [f64; 3] {
    const FORMAT: Format = Format::RGB64Sfloat;
}

impl FormatElement for [u8; 4] {
    const FORMAT: Format = Format::RGBA8Uint;
}

impl FormatElement for [Srgb; 4] {
    const FORMAT: Format = Format::RGBA8Srgb;
}

impl FormatElement for [Norm<u8>; 4] {
    const FORMAT: Format = Format::RGBA8Unorm;
}

impl FormatElement for [Scaled<u8>; 4] {
    const FORMAT: Format = Format::RGBA8Uscaled;
}

impl FormatElement for [u16; 4] {
    const FORMAT: Format = Format::RGBA16Uint;
}

impl FormatElement for [Norm<u16>; 4] {
    const FORMAT: Format = Format::RGBA16Unorm;
}

impl FormatElement for [Scaled<u16>; 4] {
    const FORMAT: Format = Format::RGBA16Uscaled;
}

impl FormatElement for [u32; 4] {
    const FORMAT: Format = Format::RGBA32Uint;
}

impl FormatElement for [u64; 4] {
    const FORMAT: Format = Format::RGBA64Uint;
}

impl FormatElement for [i8; 4] {
    const FORMAT: Format = Format::RGBA8Sint;
}

impl FormatElement for [Norm<i8>; 4] {
    const FORMAT: Format = Format::RGBA8Snorm;
}

impl FormatElement for [Scaled<i8>; 4] {
    const FORMAT: Format = Format::RGBA8Sscaled;
}

impl FormatElement for [i16; 4] {
    const FORMAT: Format = Format::RGBA16Sint;
}

impl FormatElement for [Norm<i16>; 4] {
    const FORMAT: Format = Format::RGBA16Snorm;
}

impl FormatElement for [Scaled<i16>; 4] {
    const FORMAT: Format = Format::RGBA16Sscaled;
}

impl FormatElement for [i32; 4] {
    const FORMAT: Format = Format::RGBA32Sint;
}

impl FormatElement for [i64; 4] {
    const FORMAT: Format = Format::RGBA64Sint;
}

impl FormatElement for [f32; 4] {
    const FORMAT: Format = Format::RGBA32Sfloat;
}

impl FormatElement for [f64; 4] {
    const FORMAT: Format = Format::RGBA64Sfloat;
}
