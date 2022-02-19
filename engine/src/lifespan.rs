use crate::{
    clocks::TimeSpan,
    system::{System, SystemContext},
};

/// Component for entities with limited lifespan.
#[repr(transparent)]
pub struct LifeSpan {
    pub left: TimeSpan,
}

impl LifeSpan {
    pub fn new(span: TimeSpan) -> Self {
        LifeSpan { left: span }
    }

    // Shortens lifetime to specified span.
    pub fn truncate(&mut self, span: TimeSpan) {
        self.left = std::cmp::min(self.left, span);
    }
}

pub struct LifeSpanSystem;

impl System for LifeSpanSystem {
    fn name(&self) -> &str {
        "LifeSpan"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        let mut despawn = Vec::new_in(&*cx.scope);

        for (e, ls) in cx.world.query_mut::<&mut LifeSpan>() {
            if ls.left > cx.clock.delta {
                ls.left -= cx.clock.delta;
            } else {
                despawn.push(e);
            }
        }

        for e in despawn {
            let _ = cx.world.despawn(&e);
        }
    }
}
