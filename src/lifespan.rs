use {
    crate::{
        clocks::TimeSpan,
        system::{System, SystemContext},
    },
    bumpalo::collections::Vec as BVec,
};

/// Component for entities with limited lifespan.
#[repr(transparent)]
pub struct LifeSpan {
    left: TimeSpan,
}

impl LifeSpan {
    pub fn new(span: TimeSpan) -> Self {
        LifeSpan { left: span }
    }
}

pub struct LifeSpanSystem;

impl System for LifeSpanSystem {
    fn name(&self) -> &str {
        "LifeSpan"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let mut despawn = BVec::new_in(cx.bump);

        for (e, ls) in cx.world.query_mut::<&mut LifeSpan>() {
            if ls.left > cx.clock.delta {
                ls.left -= cx.clock.delta;
            } else {
                despawn.push(e);
            }
        }

        for e in despawn {
            let _ = cx.world.despawn(e);
        }

        Ok(())
    }
}
