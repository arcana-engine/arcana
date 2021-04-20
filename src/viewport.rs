use {
    crate::{
        camera::Camera3d,
        event::{Event, WindowEvent},
        funnel::Funnel,
        graphics::Graphics,
        resources::Res,
    },
    hecs::{Entity, World},
    sierra::{Format, ImageUsage, PresentMode, Surface, SurfaceError, Swapchain, SwapchainImage},
    winit::window::{Window, WindowId},
};

/// Viewport into the world.
pub struct Viewport {
    camera: Entity,
    window: WindowId,
    surface: Surface,
    swapchain: Swapchain,
    needs_redraw: bool,
}

impl Viewport {
    /// Returns new viewport instance attached to specified camera.
    pub fn new(camera: Entity, window: &Window, graphics: &mut Graphics) -> eyre::Result<Self> {
        let mut surface = graphics.create_surface(window)?;
        let mut swapchain = graphics.create_swapchain(&mut surface)?;
        swapchain.configure(
            ImageUsage::COLOR_ATTACHMENT,
            Format::BGRA8Srgb,
            PresentMode::Fifo,
        )?;

        Ok(Viewport {
            camera,
            window: window.id(),
            surface,
            swapchain,
            needs_redraw: true,
        })
    }

    pub fn camera(&self) -> Entity {
        self.camera
    }

    /// Checks if this viewport needs a redraw.
    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    pub fn acquire_image(&mut self, optimal: bool) -> Result<SwapchainImage, SurfaceError> {
        let image = self.swapchain.acquire_image(optimal)?;
        self.needs_redraw = false;
        Ok(image)
    }
}

impl Funnel<Event> for Viewport {
    fn filter(&mut self, _res: &mut Res, world: &mut World, event: Event) -> Option<Event> {
        match event {
            Event::RedrawRequested(id) if id == self.window => {
                self.needs_redraw = true;
                None
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                if let Ok(mut camera) = world.get_mut::<Camera3d>(self.camera) {
                    camera.set_aspect(size.width as f32 / size.height as f32);
                }

                // TODO: Update for Camera2d
                Some(event)
            }
            event => Some(event),
        }
    }
}
