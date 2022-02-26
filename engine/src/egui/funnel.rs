use edict::world::World;

use crate::{event::Event, funnel::Funnel, resources::Res};

use super::EguiResource;

/// Funnel to be installed into stack,
/// that feeds events to egui.
pub struct EguiFunnel;

impl Funnel<Event> for EguiFunnel {
    fn filter(&mut self, res: &mut Res, _world: &mut World, event: Event) -> Option<Event> {
        if let Some(res) = res.get_mut::<EguiResource>() {
            if let Event::WindowEvent { event, .. } = &event {
                if res.on_event(event) {
                    return None;
                }
            }
        }
        Some(event)
    }
}
