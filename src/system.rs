use {
    crate::{clocks::ClockIndex, control::Control, prefab::PrefabLoader, resources::Res},
    bumpalo::Bump,
    hecs::World,
    std::time::{Duration, Instant},
};

/// Context in which [`System`] runs.
pub struct SystemContext<'a> {
    /// Main world.
    pub world: &'a mut World,

    /// Resources map.
    pub res: &'a mut Res,

    /// Input controllers.
    pub control: &'a mut Control,

    /// Prefab loader.
    pub loader: &'a PrefabLoader,

    /// Bump allocator.
    pub bump: &'a Bump,

    /// Clock index.
    pub clock: ClockIndex,
}

/// System trait for the ECS.
pub trait System: 'static {
    fn name(&self) -> &str;

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()>;
}

struct FixSystem<S: ?Sized> {
    step: Duration,
    next: Instant,
    system: S,
}

pub struct Scheduler {
    var_systems: Vec<Box<dyn System>>,
    fix_systems: Vec<Box<FixSystem<dyn System>>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            var_systems: Vec::new(),
            fix_systems: Vec::new(),
        }
    }

    /// Adds system to the app.
    pub fn with_system(mut self, system: impl System) -> Self {
        self.var_systems.push(Box::new(system));
        self
    }

    /// Adds system to the app.
    pub fn add_system(&mut self, system: impl System) -> &mut Self {
        self.var_systems.push(Box::new(system));
        self
    }

    /// Adds fixed-step system to the app.
    pub fn with_fixed_system(mut self, system: impl System, step: Duration) -> Self {
        self.fix_systems.push(Box::new(FixSystem {
            step,
            next: Instant::now(),
            system,
        }));
        self
    }

    /// Adds fixed-step system to the app.
    pub fn add_fixed_system(&mut self, system: impl System, step: Duration) -> &mut Self {
        self.fix_systems.push(Box::new(FixSystem {
            step,
            next: Instant::now(),
            system,
        }));
        self
    }

    pub fn start(&mut self, start: Instant) {
        for fixed in &mut self.fix_systems {
            fixed.next = start;
        }
    }

    pub fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let clock = cx.clock;

        'fixed: loop {
            let mut cx = SystemContext {
                res: cx.res,
                world: cx.world,
                control: cx.control,
                loader: cx.loader,
                bump: cx.bump,
                clock: cx.clock,
            };

            if let Some(fixed) = self.fix_systems.iter_mut().min_by_key(|f| f.next) {
                if fixed.next <= clock.current {
                    cx.clock.delta = fixed.step;
                    cx.clock.current = fixed.next;
                    fixed.system.run(cx)?;

                    fixed.next += fixed.step;
                    continue 'fixed;
                }
            }

            break;
        }

        for system in self.var_systems.iter_mut() {
            let cx = SystemContext {
                res: cx.res,
                world: cx.world,
                control: cx.control,
                loader: cx.loader,
                bump: cx.bump,
                clock: cx.clock,
            };
            system.run(cx)?;
        }

        Ok(())
    }
}
