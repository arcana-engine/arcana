use edict::{component::Component, relation::Relation};
use sierra::{Image, Rect, Surface, Swapchain};

#[derive(Component)]
pub struct RenderTarget {
    image: Option<Image>,
}

impl RenderTarget {
    pub(crate) fn new(image: Image) -> Self {
        RenderTarget { image: Some(image) }
    }

    pub(crate) fn new_swapchain() -> Self {
        RenderTarget { image: None }
    }

    pub(crate) fn set_swapchain_image(&mut self, image: Image) {
        self.image = Some(image);
    }

    pub(crate) fn clear_swapchain_image(&mut self) {
        self.image = None;
    }

    pub fn get(&self) -> Option<&Image> {
        self.image.as_ref()
    }
}

/// Relation between renderer and render target.
#[derive(Clone, Copy, Debug, Relation)]
#[edict(owned)] // When all render targets of the renderer are despawned, render node is despawned as well.
pub struct RendersTo {
    pub rect: Rect,
    pub layer: u32,
}

#[derive(Component)]
pub struct SurfaceSwapchain {
    pub surface: Surface,
    pub swapchain: Swapchain,
}

impl SurfaceSwapchain {
    pub fn new(surface: Surface, swapchain: Swapchain) -> Self {
        SurfaceSwapchain { surface, swapchain }
    }
}

/// Component that should be touched when window needs redraw.
#[derive(Component)]
pub struct NeedsRedraw;
