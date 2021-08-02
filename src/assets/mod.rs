//! Asset loading facility.

pub mod image;
pub mod object;
pub mod sprite_sheet;
pub mod tiles;

pub use self::{
    image::ImageAsset,
    sprite_sheet::{SpriteAnimation, SpriteFrame, SpriteRect, SpriteSheet, SpriteSize},
    tiles::{Tile, TileMap, TileSet},
};
