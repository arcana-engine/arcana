use bytemuck::{Pod, Zeroable};

use crate::rect::Rect;

/// Sprite configuration.
///
/// |-------------|
/// | world       |
/// |  |--------| |
/// |  |src     | |
/// |  |        | |
/// |  |--------| |
/// |-------------|
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
#[repr(C)]
pub struct Sprite {
    /// Target rect to render this sprite into.
    pub world: Rect,

    /// Specifies fraction of `world` rect that will be occupied be texture.
    pub src: Rect,

    /// Cropped rect of the sprite's texture portion.
    pub tex: Rect,

    /// Layer at which sprite should be rendered
    /// The higher level sprites are rendered over
    /// lower layer sprites.
    pub layer: u32,
}
