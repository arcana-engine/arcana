use edict::{prelude::ActionEncoder, system::Res, world::QueryRef};

use crate::{
    clocks::{ClockIndex, TimeSpan},
    scoped_allocator::ScopedAllocator,
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

pub fn lifetime_system(
    clock: Res<ClockIndex>,
    query: QueryRef<&mut LifeSpan>,
    encoder: &mut ActionEncoder,
    scope: &mut ScopedAllocator,
) {
    let mut despawn = Vec::new_in(&**scope);

    for (e, ls) in query {
        if ls.left > clock.delta {
            ls.left -= clock.delta;
        } else {
            despawn.push(e);
        }
    }

    for e in despawn {
        encoder.despawn(e);
    }
}
