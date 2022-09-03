use std::{any::TypeId, ptr::NonNull};

use arcana_time::{TimeSpan, TimeStamp};
use edict::{
    archetype::Archetype,
    query::Access,
    system::{ActionQueue, IntoSystem, System},
    world::World,
};

use crate::clocks::ClockIndex;

/// Default value for fixed systems tick_span
pub const DEFAULT_TICK_SPAN: TimeSpan = TimeSpan::from_micros(20_000);

pub struct FixSystem<S> {
    system: S,
    step: TimeSpan,
    next: Option<TimeStamp>,
}

pub trait ToFixSystem<M>: IntoSystem<M> {
    fn to_fix_system(self, step: TimeSpan) -> FixSystem<Self::System>;
}

impl<M, S> ToFixSystem<M> for S
where
    S: IntoSystem<M>,
{
    #[inline]
    fn to_fix_system(self, step: TimeSpan) -> FixSystem<Self::System> {
        FixSystem {
            system: self.into_system(),
            step,
            next: None,
        }
    }
}

impl<S> FixSystem<S> {
    /// Wraps a system to be run at a fixed rate.
    #[inline]
    pub fn new<Marker>(system: impl IntoSystem<Marker, System = S>, step: TimeSpan) -> Self {
        FixSystem {
            system: system.into_system(),
            step,
            next: None,
        }
    }
}

unsafe impl<S> System for FixSystem<S>
where
    S: System,
{
    #[inline]
    fn is_local(&self) -> bool {
        self.system.is_local()
    }

    #[inline]
    fn world_access(&self) -> Option<Access> {
        match self.system.world_access() {
            Some(Access::Write) => Some(Access::Write),
            _ => Some(Access::Read),
        }
    }

    #[inline]
    fn skips_archetype(&self, archetype: &Archetype) -> bool {
        self.system.skips_archetype(archetype)
    }

    #[inline]
    fn access_component(&self, id: TypeId) -> Option<Access> {
        self.system.access_component(id)
    }

    #[inline]
    fn access_resource(&self, id: TypeId) -> Option<Access> {
        if TypeId::of::<ClockIndex>() == id {
            // Bumps access to `Write`.
            // Reference is invalidated before inner system run.
            //
            // Warning: fixed systems are all conflicting now.
            return Some(Access::Write);
        }

        self.system.access_resource(id)
    }

    #[inline]
    unsafe fn run_unchecked(&mut self, world: NonNull<World>, queue: &mut dyn ActionQueue) {
        let clock = *world.as_ref().expect_resource::<ClockIndex>();

        let next = self.next.get_or_insert(clock.now);

        // Run systems for game ticks.
        while *next <= clock.now {
            {
                // Tweak clocks.
                let mut clock = world.as_ref().expect_resource_mut::<ClockIndex>();
                clock.delta = self.step;
                clock.now = *next;
                *next += self.step;
            }

            self.system.run_unchecked(world, queue);
        }

        // Restore clocks.
        *world.as_ref().expect_resource_mut() = clock;
    }
}
