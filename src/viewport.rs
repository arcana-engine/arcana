use crate::{
    event::{Event, WindowEvent},
    funnel::Funnel,
    graphics::Graphics,
    resources::Res,
};
use edict::{entity::EntityId, world::World};
use sierra::{Format, ImageUsage, PresentMode, Surface, SurfaceError, Swapchain, SwapchainImage};
use winit::window::{Window, WindowId};

use winit::dpi::PhysicalSize;

#[cfg(feature = "2d")]
use crate::camera::Camera2;

#[cfg(feature = "3d")]
use crate::camera::Camera3;

/// Viewport into the world.
pub struct Viewport {
    camera: EntityId,
    window: WindowId,
    #[allow(unused)]
    surface: Surface,
    swapchain: Swapchain,
    needs_redraw: bool,
    focused: bool,
    size: PhysicalSize<u32>,
    scale_factor: f64,
    swapchain_suboptimal_counter: u32,
}

const MAX_SUBOPTIMAL_SEQ: u32 = 5;

pub struct ViewportData {
    pub camera: EntityId,
    pub window: WindowId,
    pub size: PhysicalSize<u32>,
    pub scale_factor: f64,
}

pub struct Viewports {
    array: Vec<ViewportData>,
}

impl Viewports {
    const fn new() -> Self {
        Viewports { array: Vec::new() }
    }

    pub fn get(&self, window: WindowId) -> Option<&ViewportData> {
        self.array.iter().find(|data| data.window == window)
    }

    pub fn get_by_camera(&self, camera: EntityId) -> Option<&ViewportData> {
        self.array.iter().find(|data| data.camera == camera)
    }

    fn get_mut(&mut self, window: WindowId) -> Option<&mut ViewportData> {
        self.array.iter_mut().find(|data| data.window == window)
    }
}

impl Viewport {
    /// Returns new viewport instance attached to specified camera.
    pub fn new(
        camera: EntityId,
        window: &Window,
        res: &mut Res,
        graphics: &Graphics,
    ) -> eyre::Result<Self> {
        let mut surface = graphics.create_surface(window)?;
        let mut swapchain = graphics.create_swapchain(&mut surface)?;
        swapchain.configure(
            ImageUsage::COLOR_ATTACHMENT,
            Format::BGRA8Srgb,
            PresentMode::Fifo,
        )?;

        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        res.with(Viewports::new).array.push(ViewportData {
            camera,
            window: window.id(),
            size,
            scale_factor,
        });

        Ok(Viewport {
            camera,
            window: window.id(),
            surface,
            swapchain,
            needs_redraw: true,
            focused: true,
            size,
            scale_factor,
            swapchain_suboptimal_counter: 0,
        })
    }

    pub fn set_camera(&mut self, camera: EntityId) {
        self.camera = camera;
    }

    pub fn camera(&self) -> EntityId {
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

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn acquire_image(&mut self) -> Result<SwapchainImage, SurfaceError> {
        if self.swapchain_suboptimal_counter > MAX_SUBOPTIMAL_SEQ {
            self.swapchain.update()?;
            self.swapchain_suboptimal_counter = 0;
        }

        let image = self.swapchain.acquire_image()?;
        self.needs_redraw = false;

        if image.is_optimal() {
            self.swapchain_suboptimal_counter = 0;
        } else {
            self.swapchain_suboptimal_counter += 1;
        }

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
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                window_id,
            } if window_id == self.window => {
                self.size = size;

                #[cfg(any(feature = "2d", feature = "3d"))]
                let aspect = size.width as f32 / size.height as f32;

                #[cfg(feature = "2d")]
                if let Ok(camera) = world.query_one_mut::<&mut Camera2>(&self.camera) {
                    camera.set_aspect(aspect);
                }

                #[cfg(feature = "3d")]
                if let Ok(camera) = world.query_one_mut::<&mut Camera3>(&self.camera) {
                    camera.set_aspect(aspect);
                }

                res.get_mut::<Viewports>()
                    .unwrap()
                    .get_mut(self.window)
                    .unwrap()
                    .size = size;
            }
            Event::WindowEvent {
                event: WindowEvent::ScaleFactorChanged { scale_factor, .. },
                window_id,
            } if window_id == self.window => {
                self.scale_factor = scale_factor;

                res.get_mut::<Viewports>()
                    .unwrap()
                    .get_mut(self.window)
                    .unwrap()
                    .scale_factor = scale_factor;
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(focused),
                window_id,
            } if window_id == self.window => {
                self.focused = focused;
            }
            _ => {}
        }

        Some(event)
    }
}
