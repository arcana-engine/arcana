use {
    crate::{
        event::{Event, WindowEvent},
        funnel::Funnel,
        graphics::Graphics,
        resources::Res,
    },
    hecs::{Entity, World},
    sierra::{Format, ImageUsage, PresentMode, Surface, SurfaceError, Swapchain, SwapchainImage},
    winit::window::{Window, WindowId},
};

use winit::dpi::PhysicalSize;

#[cfg(feature = "2d")]
use crate::camera::Camera2;

#[cfg(feature = "3d")]
use crate::camera::Camera3;

#[cfg(feature = "sigils")]
use sigils::Ui;

/// Viewport into the world.
pub struct Viewport {
    camera: Entity,
    window: WindowId,
    #[allow(unused)]
    surface: Surface,
    swapchain: Swapchain,
    needs_redraw: bool,
    focused: bool,
    size: PhysicalSize<u32>,
}

impl Viewport {
    /// Returns new viewport instance attached to specified camera.
    pub fn new(camera: Entity, window: &Window, graphics: &Graphics) -> eyre::Result<Self> {
        let mut surface = graphics.create_surface(window)?;
        let mut swapchain = graphics.create_swapchain(&mut surface)?;
        swapchain.configure(
            ImageUsage::COLOR_ATTACHMENT,
            Format::BGRA8Srgb,
            PresentMode::Fifo,
        )?;

        let size = window.inner_size();

        Ok(Viewport {
            camera,
            window: window.id(),
            surface,
            swapchain,
            needs_redraw: true,
            focused: true,
            size,
        })
    }

    pub fn set_camera(&mut self, camera: Entity) {
        self.camera = camera;
    }

    pub fn camera(&self) -> Entity {
        self.camera
    }

    /// Checks if this viewport needs a redraw.
    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    /// Checks if viewport window is focused.
    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub fn acquire_image(&mut self, optimal: bool) -> Result<SwapchainImage, SurfaceError> {
        let image = self.swapchain.acquire_image(optimal)?;
        self.needs_redraw = false;
        Ok(image)
    }
}

impl Funnel<Event> for Viewport {
    fn filter(&mut self, res: &mut Res, world: &mut World, event: Event) -> Option<Event> {
        let _ = &world;
        let _ = &res;
        match event {
            Event::RedrawRequested(id) if id == self.window => {
                self.needs_redraw = true;
                None
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                window_id,
            } if window_id == self.window => {
                self.size = size;

                #[cfg(any(feature = "2d", feature = "3d"))]
                let aspect = size.width as f32 / size.height as f32;

                #[cfg(feature = "2d")]
                if let Ok(mut camera) = world.get_mut::<Camera2>(self.camera) {
                    camera.set_aspect(aspect);
                }

                #[cfg(feature = "3d")]
                if let Ok(mut camera) = world.get_mut::<Camera3>(self.camera) {
                    camera.set_aspect(aspect);
                }

                #[cfg(feature = "sigils")]
                if let Some(ui) = res.get_mut::<Ui>() {
                    ui.set_extent(sigils::Vector2 {
                        x: size.width as f32,
                        y: size.height as f32,
                    });
                }

                Some(event)
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(focused),
                window_id,
            } if window_id == self.window => {
                self.focused = focused;
                None
            }
            event => Some(event),
        }
    }
}
