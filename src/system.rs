use {
    crate::{
        clocks::{ClockIndex, TimeSpan, TimeStamp},
        control::Control,
        graphics::Graphics,
        resources::Res,
        task::{Spawner, TaskContext},
    },
    eyre::WrapErr as _,
    goods::Loader,
    hecs::World,
    scoped_arena::Scope,
};

/// Default value for fixed systems tick_span
pub const DEFAULT_TICK_SPAN: TimeSpan = TimeSpan::from_micros(20_000);

/// Context in which [`System`] runs.
///
/// `System::run` accepts this struct as argument.
/// `SystemContext` contains everything system would need to run.
pub struct SystemContext<'a> {
    /// World on which systems are run.
    pub world: &'a mut World,

    /// Resources map.
    /// All singleton values are stored here and accessible by type.
    pub res: &'a mut Res,

    /// Control hub to make entities controlled.
    pub control: &'a mut Control,

    /// Spawns tasks that will be executed asynchronously.
    pub spawner: &'a mut Spawner,

    /// Graphics context.
    pub graphics: &'a mut Graphics,

    /// Asset loader.
    /// Assets are loaded asynchronously,
    /// result can be awaited in task. See `spawner` field.
    pub loader: &'a Loader,

    /// Arena allocator for allocations in hot-path.
    pub scope: &'a mut Scope<'static>,

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
            scope: self.scope,
            clock: self.clock,
        }
    }

    /// Reborrow as task context.
    pub fn task(&mut self) -> TaskContext<'_> {
        TaskContext {
            world: self.world,
            res: self.res,
            control: self.control,
            spawner: self.spawner,
            graphics: self.graphics,
            loader: self.loader,
            scope: &self.scope,
        }
    }
}

/// System trait for the ECS.
pub trait System: 'static {
    /// Name of the system.
    /// Used for debug purposes.
    fn name(&self) -> &str;

    /// Run system with provided context.
    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()>;
}

/// Functions are systems.
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

struct FixSystem {
    system: Box<dyn System>,
    step: TimeSpan,
    next: TimeStamp,
}

pub struct Scheduler {
    var_systems: Vec<Box<dyn System>>,
    fixed_systems: Vec<FixSystem>,
    tick_systems: Vec<Box<dyn System>>,
    tick_span: TimeSpan,
    next_tick: TimeStamp,
}

impl Scheduler {
    /// Creates new scheduler instance with default tick step: [`DEFAULT_TICK_SPAN`].
    pub fn new() -> Self {
        Scheduler::with_tick_span(DEFAULT_TICK_SPAN)
    }

    /// Creates new scheduler with specified tick step.
    pub fn with_tick_span(tick_span: TimeSpan) -> Self {
        Scheduler {
            var_systems: Vec::new(),
            fixed_systems: Vec::new(),
            tick_systems: Vec::new(),
            next_tick: TimeStamp::ORIGIN,
            tick_span,
        }
    }

    pub fn set_tick_span(&mut self, tick_span: TimeSpan) {
        self.tick_span = tick_span;
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
        self.fixed_systems.push(FixSystem {
            system: Box::new(system),
            next: TimeStamp::ORIGIN,
            step,
        });
        self
    }

    /// Adds fixed-step system to the app.
    pub fn add_fixed_system(&mut self, system: impl System, step: TimeSpan) -> &mut Self {
        self.fixed_systems.push(FixSystem {
            system: Box::new(system),
            next: TimeStamp::ORIGIN,
            step,
        });
        self
    }

    /// Adds ticking system to the app.
    pub fn with_ticking_system(mut self, system: impl System) -> Self {
        self.tick_systems.push(Box::new(system));
        self
    }

    /// Adds ticking system to the app.
    pub fn add_ticking_system(&mut self, system: impl System) -> &mut Self {
        self.tick_systems.push(Box::new(system));
        self
    }

    pub fn start(&mut self, start: TimeStamp) {
        self.next_tick = start;

        for fixed in &mut self.fixed_systems {
            fixed.next = start;
        }
    }

    pub fn run(&mut self, mut cx: SystemContext<'_>) -> eyre::Result<()> {
        let clock = cx.clock;

        // Run systems for game ticks.
        while self.next_tick <= clock.now {
            cx.clock.delta = self.tick_span;
            cx.clock.now = self.next_tick;

            for system in self.tick_systems.iter_mut() {
                let cx = cx.reborrow();
                system.run(cx).wrap_err_with(|| SystemFailure {
                    name: system.name().to_owned(),
                })?;
            }

            self.next_tick += self.tick_span;
        }

        // Run systems with fixed step.
        loop {
            match self.fixed_systems.iter_mut().min_by_key(|fixed| fixed.next) {
                None => break,
                Some(fixed) if fixed.next > clock.now => break,
                Some(fixed) => {
                    cx.clock.delta = fixed.step;
                    cx.clock.now = fixed.next;

                    let cx = cx.reborrow();
                    fixed.system.run(cx).wrap_err_with(|| SystemFailure {
                        name: fixed.system.name().to_owned(),
                    })?;

                    fixed.next += fixed.step;
                }
            }
        }

        // Run variable rate systems.
        cx.clock = clock;

        for system in self.var_systems.iter_mut() {
            let cx = cx.reborrow();
            system.run(cx).wrap_err_with(|| SystemFailure {
                name: system.name().to_owned(),
            })?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("System {name} execution failed")]
pub struct SystemFailure {
    name: String,
}
