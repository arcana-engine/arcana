use {
    crate::{
        camera::{Camera2, Camera3},
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
    focused: bool,
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

        Ok(Viewport {
            camera,
            window: window.id(),
            surface,
            swapchain,
            needs_redraw: true,
            focused: true,
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
                window_id,
            } if window_id == self.window => {
                let aspect = size.width as f32 / size.height as f32;
                if let Ok(mut camera) = world.get_mut::<Camera3>(self.camera) {
                    camera.set_aspect(aspect);
                }
                if let Ok(mut camera) = world.get_mut::<Camera2>(self.camera) {
                    camera.set_aspect(aspect);
                }

                // TODO: Update for Camera2d
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
