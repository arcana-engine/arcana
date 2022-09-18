use edict::{component::Component, entity::EntityId, world::World};
use hashbrown::HashMap;
use winit::{dpi::PhysicalSize, window::WindowId};

use crate::{
    event::{Event, WindowEvent},
    funnel::Funnel,
    graphics::RenderTarget,
};

/// Window component associated with particular [`Window`].
#[derive(Component)]
pub struct Window {
    focused: bool,
    swapchain_suboptimal_counter: u32,
    window: WindowId,
    size: PhysicalSize<u32>,
    scale_factor: f64,
}

const MAX_SUBOPTIMAL_SEQ: u32 = 5;

impl Window {
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

impl Window {
    /// Checks if viewport window is focused.
    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn aspect(&self) -> f32 {
        self.size.width as f32 / self.size.height as f32
    }

    pub fn create_render_target(&self, world: &mut World) {}
}

/// Even listener for window events.
pub struct Windows {
    windows: HashMap<WindowId, EntityId>,
}

impl Windows {
    pub fn new() -> Self {
        Windows {
            windows: HashMap::new(),
        }
    }

    pub fn is_focused(&self, id: WindowId, world: &World) -> Option<bool> {
        let id = self.windows.get(&id)?;
        let window = world.query_one::<&Window>(*id).ok()?;
        Some(window.get()?.focused)
    }

    pub fn skip_event(&self, event: &Event, world: &World) -> bool {
        match event {
            Event::WindowEvent { window_id, .. } => {
                !self.is_focused(*window_id, world).unwrap_or(false)
            }
            _ => false,
        }
    }

    /// Returns new viewport instance attached to specified camera.
    pub fn spawn(&mut self, window: &winit::window::Window, world: &mut World) -> EntityId {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        let window = Window {
            focused: true,
            swapchain_suboptimal_counter: 0,
            window: window.id(),
            size,
            scale_factor,
        };

        let id = world.spawn((window,));
        self.windows.insert(window.window, id);
        id
    }
}

impl Funnel<Event> for Windows {
    fn filter(&mut self, world: &mut World, event: Event) -> Option<Event> {
        match event {
            Event::RedrawRequested(id) => {
                if let Some(entity) = self.windows.get_mut(&id) {
                    // We use side-effect of mutable touching `RenderTarget` component to mark it as dirty.
                    world.query_one::<&mut RenderTarget>(*entity);
                    return None;
                }
            }
            Event::WindowEvent { event, window_id } => {
                if let Some(entity) = self.windows.get(&window_id) {
                    if let Ok(mut window) = world.query_one_mut::<&mut Window>(*entity) {
                        match event {
                            WindowEvent::Resized(size) => {
                                window.size = size;
                            }
                            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                                window.scale_factor = scale_factor;
                            }
                            WindowEvent::Focused(focused) => {
                                window.focused = focused;
                            }
                            _ => {
                                if window.focused {
                                    return Some(Event::WindowEvent { event, window_id });
                                }
                            }
                        }
                    }
                    return None;
                }
            }
            _ => {}
        }
        Some(event)
    }
}
