#![feature(allocator_api)]

#[cfg(feature = "2d")]
pub mod physics2;

#[cfg(feature = "3d")]
pub mod physics3;
