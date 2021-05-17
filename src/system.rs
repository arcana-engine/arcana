use {
    crate::{
        clocks::{ClockIndex, TimeSpan},
        control::Control,
        graphics::Graphics,
        resources::Res,
        task::{Spawner, TaskContext},
    },
    bumpalo::Bump,
    goods::Loader,
    hecs::World,
};

/// Context in which [`System`] runs.
pub struct SystemContext<'a> {
    /// Main world.
    pub world: &'a mut World,

    /// Resources map.
    pub res: &'a mut Res,

    /// Input controllers.
    pub control: &'a mut Control,

    /// Task spawner,
    pub spawner: &'a mut Spawner,

    /// Graphics context.
    pub graphics: &'a mut Graphics,

    /// Asset loader
    pub loader: &'a Loader,

    /// Bump allocator.
    pub bump: &'a Bump,

    /// Clock index.
    pub clock: ClockIndex,
}

impl<'a> SystemContext<'a> {
    /// Reborrow system context.
    pub fn reborrow(&mut self) -> SystemContext<'_> {
        SystemContext {
            world: self.world,
            res: self.res,
            control: self.control,
            spawner: self.spawner,
            graphics: self.graphics,
            loader: self.loader,
            bump: self.bump,
            clock: self.clock,
        }
    }

    pub fn task(&mut self) -> TaskContext<'_> {
        TaskContext {
            world: self.world,
            res: self.res,
            control: self.control,
            spawner: self.spawner,
            graphics: self.graphics,
            loader: self.loader,
            bump: self.bump,
        }
    }
}

/// System trait for the ECS.
pub trait System: 'static {
    fn name(&self) -> &str;

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()>;
}

impl<F> System for F
where
    F: for<'a> FnMut(SystemContext<'a>) + 'static,
{
    fn name(&self) -> &str {
        std::any::type_name::<F>()
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        (*self)(cx);
        Ok(())
    }
}

struct FixSystem<S: ?Sized> {
    step: TimeSpan,
    next: TimeSpan,
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
    pub fn with_fixed_system(mut self, system: impl System, step: TimeSpan) -> Self {
        self.fix_systems.push(Box::new(FixSystem {
            step,
            next: TimeSpan::ZERO,
            system,
        }));
        self
    }

    /// Adds fixed-step system to the app.
    pub fn add_fixed_system(&mut self, system: impl System, step: TimeSpan) -> &mut Self {
        self.fix_systems.push(Box::new(FixSystem {
            step,
            next: TimeSpan::ZERO,
            system,
        }));
        self
    }

    pub fn start(&mut self, start: TimeSpan) {
        for fixed in &mut self.fix_systems {
            fixed.next = start;
        }
    }

    pub fn run(&mut self, mut cx: SystemContext<'_>) -> eyre::Result<()> {
        let clock = cx.clock;

        'fixed: loop {
            let mut cx = cx.reborrow();

            if let Some(fixed) = self.fix_systems.iter_mut().min_by_key(|f| f.next) {
                if fixed.next <= clock.elapsed {
                    cx.clock.delta = fixed.step;
                    cx.clock.elapsed = fixed.next;
                    fixed.system.run(cx)?;

                    fixed.next += fixed.step;
                    continue 'fixed;
                }
            }

            break;
        }

        for system in self.var_systems.iter_mut() {
            let cx = cx.reborrow();
            system.run(cx)?;
        }

        Ok(())
    }
}
