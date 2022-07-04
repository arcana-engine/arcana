use crate::{
    event::{Event, WindowEvent},
    funnel::Funnel,
    graphics::Graphics,
    resources::Res,
};
use edict::{entity::EntityId, world::World};
use sierra::{ImageUsage, PresentMode, Surface, SurfaceError, Swapchain, SwapchainImage};
use winit::window::{Window, WindowId};

use winit::dpi::PhysicalSize;

#[cfg(feature = "2d")]
use crate::camera::Camera2;

#[cfg(feature = "3d")]
use crate::camera::Camera3;

/// Viewport into the world.
pub struct Viewport {
    #[allow(unused)]
    surface: Surface,
    swapchain: Swapchain,
    needs_redraw: bool,
    focused: bool,
    swapchain_suboptimal_counter: u32,
    data: ViewportData,
}

const MAX_SUBOPTIMAL_SEQ: u32 = 5;

#[derive(Clone, Copy, Debug)]
pub struct ViewportData {
    pub camera: EntityId,
    pub window: WindowId,
    pub size: PhysicalSize<u32>,
    pub scale_factor: f64,
}

impl std::ops::Deref for Viewport {
    type Target = ViewportData;

    fn deref(&self) -> &ViewportData {
        &self.data
    }
}

impl ViewportData {
    /// Converts viewport coordinates to screen space.
    pub fn viewport_to_pixel(&self, xy: [f32; 2]) -> [u32; 2] {
        let x = xy[0].clamp(0.0, self.size.width as f32) as u32;
        let y = xy[1].clamp(0.0, self.size.height as f32) as u32;
        let y = self.size.height - y;
        [x, y]
    }

    /// Converts viewport coordinates to screen space.
    pub fn viewport_to_screen(&self, xy: [f32; 2]) -> [f32; 2] {
        let x = (xy[0] / self.size.width as f32 * 2.0) - 1.0;
        let y = 1.0 - (xy[1] / self.size.height as f32 * 2.0);
        [x, y]
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
        if res.get::<ViewportData>().is_some() {
            return Err(eyre::eyre!("Only one viewport per `Res` is supported"));
        }

        let mut surface = graphics.create_surface(window)?;
        let mut swapchain = graphics.create_swapchain(&mut surface)?;

        let format = swapchain
            .capabilities()
            .formats
            .iter()
            .filter(|format| {
                format.is_color()
                    && matches!(
                        format.description().channels,
                        sierra::Channels::RGBA
                            | sierra::Channels::BGRA
                            | sierra::Channels::RGB
                            | sierra::Channels::BGR
                    )
            })
            .max_by_key(|format| match format.description().channels {
                sierra::Channels::RGBA | sierra::Channels::BGRA => 0,
                sierra::Channels::BGR | sierra::Channels::RGB => 1,
                _ => unreachable!(),
            } + match format.description().ty {
                sierra::Type::Srgb => 2,
                sierra::Type::Sint => 0,
                sierra::Type::Unorm => 1,
                sierra::Type::Snorm => 1,
                _ => 0,
            } * 2);

        match format {
            None => {
                return Err(eyre::eyre!(
                    "Failed to find suitable format. Supported formats are {:?}",
                    swapchain.capabilities().formats
                ))
            }
            Some(format) => {
                swapchain.configure(ImageUsage::COLOR_ATTACHMENT, *format, PresentMode::Fifo)?;
            }
        }

        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        let data = ViewportData {
            camera,
            window: window.id(),
            size,
            scale_factor,
        };

        res.insert(data);

        Ok(Viewport {
            data,
            surface,
            swapchain,
            needs_redraw: true,
            focused: true,
            swapchain_suboptimal_counter: 0,
        })
    }

    pub fn set_camera(&mut self, camera: EntityId) {
        self.data.camera = camera;
    }

    /// Checks if this viewport needs a redraw.
    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    /// Checks if viewport window is focused.
    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn aspect(&self) -> f32 {
        self.size.width as f32 / self.size.height as f32
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
                self.data.size = size;

                #[cfg(any(feature = "2d", feature = "3d"))]
                let aspect = self.aspect();

                #[cfg(feature = "2d")]
                if let Ok(camera) = world.query_one_mut::<&mut Camera2>(&self.camera) {
                    camera.set_aspect(aspect);
                }

                #[cfg(feature = "3d")]
                if let Ok(camera) = world.query_one_mut::<&mut Camera3>(&self.camera) {
                    camera.set_aspect(aspect);
                }

                res.get_mut::<ViewportData>().unwrap().size = size;
            }
            Event::WindowEvent {
                event: WindowEvent::ScaleFactorChanged { scale_factor, .. },
                window_id,
            } if window_id == self.window => {
                self.data.scale_factor = scale_factor;

                res.get_mut::<ViewportData>().unwrap().scale_factor = scale_factor;
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
