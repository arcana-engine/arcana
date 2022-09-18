use edict::world::World;

use crate::{event::Event, funnel::Funnel};

use super::EguiResource;

/// Funnel to be installed into stack,
/// that feeds events to egui.
pub struct EguiFunnel;

impl Funnel<Event> for EguiFunnel {
    fn filter(&mut self, world: &mut World, event: Event) -> Option<Event> {
        if let Some(mut res) = world.get_resource_mut::<EguiResource>() {
            if let Event::WindowEvent { event, .. } = &event {
                if res.on_event(event) {
                    return None;
                }
            }
        }
        Some(event)
    }
}
