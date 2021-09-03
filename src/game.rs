use {
    crate::{
        clocks::{Clocks, TimeSpan},
        control::Control,
        event::{Event, Loop, WindowEvent},
        fps::FpsMeter,
        funnel::Funnel,
        graphics::{Graphics, Renderer, RendererContext},
        lifespan::LifeSpanSystem,
        resources::Res,
        scene::SceneSystem,
        system::{Scheduler, SystemContext},
        task::{Executor, Spawner, TaskContext},
        viewport::Viewport,
    },
    eyre::WrapErr,
    goods::Loader,
    hecs::{DynamicBundle, World},
    scoped_arena::Scope,
    std::{future::Future, path::Path, time::Duration},
    winit::window::{Window, WindowBuilder},
};

#[cfg(any(feature = "2d", feature = "3d"))]
use crate::graphics::renderer::forward::ForwardRenderer;

#[cfg(feature = "2d")]
use crate::{camera::Camera2, graphics::renderer::sprite::SpriteDraw, scene::Global2};

#[cfg(feature = "3d")]
use crate::{camera::Camera3, graphics::renderer::basic::BasicDraw, scene::Global3};

#[repr(transparent)]
pub struct MainWindow {
    window: Window,
}

impl std::ops::Deref for MainWindow {
    type Target = Window;

    fn deref(&self) -> &Window {
        &self.window
    }
}

impl MainWindow {
    fn new(event_loop: &Loop) -> eyre::Result<Self> {
        Ok(MainWindow {
            window: WindowBuilder::new()
                .with_title("Arcana Game")
                .build(event_loop)?,
        })
    }
}

struct MainWindowFunnel;

impl Funnel<Event> for MainWindowFunnel {
    fn filter(&mut self, res: &mut Res, _world: &mut World, event: Event) -> Option<Event> {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } => {
                if let Some(window) = res.get::<MainWindow>() {
                    if window_id == window.id() {
                        return Some(Event::Exit);
                    }
                }
                Some(event)
            }
            Event::Loop => {
                if let Some(window) = res.get::<MainWindow>() {
                    window.request_redraw();
                }
                Some(Event::Loop)
            }
            _ => Some(event),
        }
    }
}

pub struct Game {
    pub res: Res,
    pub world: World,
    pub scheduler: Scheduler,
    pub control: Control,
    pub graphics: Graphics,
    pub renderer: Option<Box<dyn Renderer + Send>>,
    pub viewport: Viewport,
    pub loader: Loader,
    pub spawner: Spawner,
    pub scope: Scope<'static>,
}

impl Game {
    pub fn cx(&mut self) -> TaskContext<'_> {
        TaskContext {
            world: &mut self.world,
            res: &mut self.res,
            control: &mut self.control,
            spawner: &mut self.spawner,
            graphics: &mut self.graphics,
            loader: &mut self.loader,
            scope: &mut self.scope,
        }
    }
}

#[cfg(feature = "2d")]
pub fn game2<F, Fut>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    tracing::debug!("Starting 2D game");
    game::<_, _, _, (Camera2, Global2)>(f, |g| {
        Ok(Box::new(ForwardRenderer::new(SpriteDraw::new(g)?)))
    })
}

#[cfg(feature = "3d")]
pub fn game3<F, Fut>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    tracing::debug!("Starting 3D game");
    game::<_, _, _, (Camera3, Global3)>(f, |g| {
        Ok(Box::new(ForwardRenderer::new(BasicDraw::new(g)?)))
    })
}

fn game<F, Fut, R, C>(f: F, r: R)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
    R: FnOnce(&mut Graphics) -> eyre::Result<Box<dyn Renderer>> + Send + 'static,
    C: DynamicBundle + Default,
{
    crate::install_eyre_handler();
    crate::install_tracing_subscriber();

    Loop::run(|event_loop| async move {
        // Load config.
        let cfg = load_default_config();

        // Initialize asset loader.
        let mut loader_builder = Loader::builder();
        if let Some(path) = cfg.treasury {
            let treasury = goods::source::treasury::TreasurySource::open(path)
                .wrap_err_with(|| "Failed to initialize treasury loader")?;
            loader_builder.add(treasury);
        }
        let loader = loader_builder.build();

        // Create new world with camera.
        let mut world = World::new();

        // Open game window.
        let window =
            MainWindow::new(&event_loop).wrap_err_with(|| "Failed to initialize main window")?;

        let window_size = window.inner_size();

        let camera = world.spawn(C::default());

        #[cfg(any(feature = "2d", feature = "3d"))]
        let aspect = window_size.width as f32 / window_size.height as f32;

        #[cfg(feature = "2d")]
        if let Ok(mut camera) = world.get_mut::<Camera2>(camera) {
            camera.set_aspect(aspect);
        }

        #[cfg(feature = "3d")]
        if let Ok(mut camera) = world.get_mut::<Camera3>(camera) {
            camera.set_aspect(aspect);
        }

        // Initialize graphics system.
        let graphics = Graphics::new().wrap_err_with(|| "Failed to initialize graphics")?;

        // Attach viewport to window and camera.
        let viewport = Viewport::new(camera, &window, &graphics)
            .wrap_err_with(|| "Failed to initialize main viewport")?;

        let spawner = Spawner::new();
        let mut res = Res::new();
        res.insert(window);

        // Configure game with closure.
        let game = f(Game {
            res,
            world,
            scheduler: Scheduler::new(),
            control: Control::new(),
            graphics,
            renderer: None,
            viewport,
            loader,
            spawner,
            scope: Scope::new(),
        })
        .await
        .wrap_err_with(|| "Game startup failed")?;

        let Game {
            mut res,
            mut world,
            mut scheduler,
            mut control,
            mut graphics,
            renderer,
            mut viewport,
            mut loader,
            mut spawner,
            mut scope,
        } = game;

        scope.reset();

        // Take renderer. Use default one if not configured.
        let mut renderer = match renderer {
            Some(renderer) => renderer,
            None => r(&mut graphics).wrap_err_with(|| "Renderer build failed")?,
        };

        // Start the clocks.
        let mut clocks = Clocks::new();

        // Schedule default systems.
        scheduler.add_system(LifeSpanSystem);
        scheduler.add_system(SceneSystem);

        res.insert(FpsMeter::new(TimeSpan::SECOND));
        scheduler.add_fixed_system(
            |cx: SystemContext<'_>| {
                let fps = cx.res.get::<FpsMeter>().unwrap();
                println!("FPS: {}", fps.fps());
            },
            TimeSpan::SECOND,
        );

        let mut executor = Executor::new();

        // Begin game loop.
        loop {
            // Loop through new  events.
            let mut funnel = GameFunnel {
                viewport: &mut viewport,
                control: &mut control,
            };

            loop {
                let event = event_loop.next_event(Duration::new(0, 1_000_000)).await;

                // Filter event
                let event = funnel.filter(&mut res, &mut world, event);

                match event {
                    Some(Event::Loop) => break, // No new events. Continue game loop
                    Some(Event::Exit) => {
                        // It's time to exit. This event never generated by event loop.
                        // For example viewport generates this event on windows close.

                        // Try to finish outstanding async tasks.
                        executor
                            .teardown(
                                TaskContext {
                                    world: &mut world,
                                    res: &mut res,
                                    control: &mut control,
                                    spawner: &mut spawner,
                                    graphics: &mut graphics,
                                    loader: &mut loader,
                                    scope: &mut scope,
                                },
                                cfg.teardown_timeout,
                            )
                            .await;

                        drop(renderer);
                        drop(world);

                        // Wait for graphics to finish pending work.
                        graphics.wait_idle();
                        return Ok(());
                    }
                    _ => {}
                }
            }

            let clock = clocks.advance();

            scheduler
                .run(SystemContext {
                    world: &mut world,
                    res: &mut res,
                    control: &mut control,
                    spawner: &mut spawner,
                    graphics: &mut graphics,
                    loader: &mut loader,
                    scope: &mut scope,
                    clock,
                })
                .wrap_err_with(|| "System returned error")?;

            executor.append(&mut spawner);
            executor
                .run_once(TaskContext {
                    world: &mut world,
                    res: &mut res,
                    control: &mut control,
                    spawner: &mut spawner,
                    graphics: &mut graphics,
                    loader: &mut loader,
                    scope: &mut scope,
                })
                .wrap_err_with(|| "Task returned error")?;

            graphics
                .flush_uploads(&scope)
                .wrap_err_with(|| "Uploads failed")?;

            res.get_mut::<FpsMeter>()
                .unwrap()
                .add_frame_time(clock.delta);

            renderer
                .render(
                    RendererContext {
                        world: &mut world,
                        res: &mut res,
                        graphics: &mut graphics,
                        scope: &scope,
                        clock,
                    },
                    &mut [&mut viewport],
                )
                .wrap_err_with(|| "Renderer failed")?;

            scope.reset();
        }
    });
}

struct GameFunnel<'a> {
    viewport: &'a mut Viewport,
    control: &'a mut Control,
}

impl Funnel<Event> for GameFunnel<'_> {
    fn filter(&mut self, res: &mut Res, world: &mut World, event: Event) -> Option<Event> {
        match Funnel::filter(&mut MainWindowFunnel, res, world, event) {
            None => None,
            Some(event) => match Funnel::filter(&mut *self.viewport, res, world, event) {
                None => None,
                Some(event) if self.viewport.focused() => {
                    Funnel::filter(&mut *self.control, res, world, event)
                }
                Some(event) => Some(event),
            },
        }
    }
}

#[derive(Default, serde::Deserialize)]
struct Config {
    #[serde(default)]
    treasury: Option<Box<Path>>,

    #[serde(default = "default_teardown_timeout")]
    teardown_timeout: Duration,
}

fn default_teardown_timeout() -> Duration {
    Duration::from_secs(5)
}

#[tracing::instrument]
fn load_config(path: &Path) -> eyre::Result<Config> {
    let cfg = std::fs::read(path)?;
    let cfg = serde_json::from_slice(&cfg)?;
    Ok(cfg)
}

fn try_load_default_config() -> eyre::Result<Config> {
    tracing::debug!("Loading config");

    let path = Path::new("cfg.json");
    if path.is_file() {
        load_config(path)
    } else {
        let mut path = std::env::current_exe()?;
        path.set_file_name("cfg.json");

        if path.is_file() {
            load_config(&path)
        } else {
            Err(eyre::eyre!("Failed to locate config file"))
        }
    }
}

fn load_default_config() -> Config {
    match try_load_default_config() {
        Ok(cfg) => cfg,
        Err(err) => {
            tracing::debug!("Config file not found. {:#}", err);
            Config::default()
        }
    }
}
