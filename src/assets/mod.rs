//! Asset loading facility.

pub mod image;
pub mod object;
pub mod sprite_sheet;

#[cfg(feature = "physics2d")]
pub mod tiles;

pub use self::{
    image::ImageAsset,
    sprite_sheet::{SpriteAnimation, SpriteFrame, SpriteRect, SpriteSheet, SpriteSize},
};

#[cfg(feature = "physics2d")]
pub use self::tiles::{Tile, TileMap, TileSet};
