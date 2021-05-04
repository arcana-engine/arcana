#[repr(C)]
#[derive(Clone, Copy)]
pub struct AABB {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Default for AABB {
    fn default() -> Self {
        AABB {
            left: 0.0,
            right: 1.0,
            top: 0.0,
            bottom: 1.0,
        }
    }
}

/// Sprite component.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Sprite {
    /// AABB of the sprite to render.
    pub pos: AABB,

    /// AABB of sprite texture portion.
    pub uv: AABB,

    /// Layer at which sprite should be rendered
    /// Layers are relative, the higher level sprites are rendered over
    /// lower layer sprites.
    pub layer: u32,
}
