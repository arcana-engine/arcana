use {
    crate::{
        clocks::{Clocks, TimeSpan},
        fps::FpsMeter,
        lifespan::LifeSpanSystem,
        resources::Res,
        system::{Scheduler, SystemContext},
        task::{Executor, Spawner, TaskContext},
    },
    eyre::WrapErr,
    goods::Loader,
    hecs::World,
    scoped_arena::Scope,
    std::{future::Future, path::Path, time::Duration},
};

#[cfg(any(feature = "2d", feature = "3d"))]
use crate::scene::SceneSystem;

#[cfg(feature = "visible")]
use {
    crate::{
        control::Control,
        event::{Event, Loop, WindowEvent},
        funnel::Funnel,
        graphics::{Graphics, Renderer, RendererContext},
        hecs::DynamicBundle,
        viewport::Viewport,
    },
    winit::window::{Window, WindowBuilder},
};

#[cfg(all(any(feature = "2d", feature = "3d"), feature = "visible"))]
use crate::graphics::renderer::simple::SimpleRenderer;

#[cfg(feature = "client")]
use evoke::client::ClientSystem;

#[cfg(feature = "server")]
use evoke::server::ServerSystem;

#[cfg(all(feature = "2d", feature = "visible"))]
use crate::{camera::Camera2, graphics::renderer::sprite::SpriteDraw, scene::Global2};

#[cfg(all(feature = "3d", feature = "visible"))]
use crate::{camera::Camera3, graphics::renderer::basic::BasicDraw, scene::Global3};

#[cfg(feature = "visible")]
#[repr(transparent)]
pub struct MainWindow {
    window: Window,
}

#[cfg(feature = "visible")]
impl std::ops::Deref for MainWindow {
    type Target = Window;

    fn deref(&self) -> &Window {
        &self.window
    }
}

#[cfg(feature = "visible")]
impl MainWindow {
    fn new(event_loop: &Loop) -> eyre::Result<Self> {
        Ok(MainWindow {
            window: WindowBuilder::new()
                .with_title("Arcana Game")
                .build(event_loop)?,
        })
    }
}

#[cfg(feature = "visible")]
struct MainWindowFunnel;

#[cfg(feature = "visible")]
impl Funnel<Event> for MainWindowFunnel {
    fn filter(&mut self, res: &mut Res, _world: &mut World, event: Event) -> Option<Event> {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } => {
                if let Some(window) = res.get::<MainWindow>() {
                    if window_id == window.id() {
                        res.insert(Exit);
                        res.remove::<MainWindow>();
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

/// Resource that when added exists game loop.
pub struct Exit;

pub struct Game {
    pub res: Res,
    pub world: World,
    pub scheduler: Scheduler,
    pub loader: Loader,
    pub spawner: Spawner,
    pub scope: Scope<'static>,

    #[cfg(feature = "visible")]
    pub control: Control,

    #[cfg(feature = "visible")]
    pub graphics: Graphics,

    #[cfg(feature = "visible")]
    pub renderer: Option<Box<dyn Renderer>>,

    #[cfg(feature = "visible")]
    pub viewport: Viewport,

    #[cfg(feature = "client")]
    pub client: Option<ClientSystem>,

    #[cfg(feature = "server")]
    pub server: Option<ServerSystem>,
}

impl Game {
    pub fn cx(&mut self) -> TaskContext<'_> {
        TaskContext {
            world: &mut self.world,
            res: &mut self.res,
            spawner: &mut self.spawner,
            loader: &mut self.loader,
            scope: &mut self.scope,
            #[cfg(feature = "visible")]
            control: &mut self.control,
            #[cfg(feature = "visible")]
            graphics: &mut self.graphics,
        }
    }
}

#[cfg(all(feature = "2d", feature = "visible"))]
pub fn game2<F, Fut>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    tracing::debug!("Starting 2D game");
    game::<_, _, _, (Camera2, Global2)>(f, |g| {
        Ok(Box::new(SimpleRenderer::new(SpriteDraw::new(0.0..1.0, g)?)))
    })
}

#[cfg(all(feature = "3d", feature = "visible"))]
pub fn game3<F, Fut>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    tracing::debug!("Starting 3D game");
    game::<_, _, _, (Camera3, Global3)>(f, |g| {
        Ok(Box::new(SimpleRenderer::new(BasicDraw::new(g)?)))
    })
}

#[cfg(feature = "visible")]
pub fn game<F, Fut, R, C>(f: F, r: R)
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
            let treasury = goods::source::treasury::TreasurySource::open_local(&path)
                .await
                .wrap_err_with(|| "Failed to initialize treasury loader")?;
            loader_builder.add(treasury);
        }
        let loader = loader_builder.build();

        // Create new world with camera.
        let mut world = World::new();

        // Open game window.
        let window =
            MainWindow::new(&event_loop).wrap_err_with(|| "Failed to initialize main window")?;

        let camera = world.spawn(C::default());

        #[cfg(any(feature = "2d", feature = "3d"))]
        {
            let window_size = window.inner_size();

            let aspect = window_size.width as f32 / window_size.height as f32;

            #[cfg(feature = "2d")]
            if let Ok(mut camera) = world.get_mut::<Camera2>(camera) {
                camera.set_aspect(aspect);
            }

            #[cfg(feature = "3d")]
            if let Ok(mut camera) = world.get_mut::<Camera3>(camera) {
                camera.set_aspect(aspect);
            }
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
            scheduler: Scheduler::with_tick_span(cfg.main_step),
            control: Control::new(),
            graphics,
            renderer: None,
            viewport,
            loader,
            spawner,
            scope: Scope::new(),

            #[cfg(feature = "client")]
            client: None,

            #[cfg(feature = "server")]
            server: None,
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

            #[cfg(feature = "client")]
            mut client,

            #[cfg(feature = "server")]
            mut server,
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
        #[cfg(any(feature = "2d", feature = "3d"))]
        scheduler.add_system(SceneSystem::new());

        scheduler.add_system(LifeSpanSystem);

        res.insert(FpsMeter::new(TimeSpan::SECOND));
        scheduler.add_fixed_system(
            |cx: SystemContext<'_>| {
                let fps = cx.res.get::<FpsMeter>().unwrap();
                tracing::info!("FPS: {}", fps.fps());
            },
            TimeSpan::SECOND,
        );

        let mut executor = Executor::new();

        let main_step = cfg.main_step;
        let mut step_ns = 0;

        // Begin game loop.
        loop {
            loop {
                // Loop through new  events.
                let mut funnel = GameFunnel {
                    viewport: &mut viewport,
                    control: &mut control,
                };

                let event = event_loop.next_event(Duration::new(0, 1_000_000)).await;

                // Filter event
                let event = funnel.filter(&mut res, &mut world, event);

                match event {
                    Some(Event::Loop) => break, // No new events. Continue game loop
                    None => {
                        executor
                            .run_once(TaskContext {
                                world: &mut world,
                                res: &mut res,
                                spawner: &mut spawner,
                                loader: &mut loader,
                                scope: &mut scope,

                                #[cfg(feature = "visible")]
                                control: &mut control,

                                #[cfg(feature = "visible")]
                                graphics: &mut graphics,
                            })
                            .wrap_err_with(|| "Task returned error")?;
                    }
                    _ => {}
                }
            }

            if res.get::<Exit>().is_some() {
                // Try to finish outstanding async tasks.
                executor
                    .teardown(
                        TaskContext {
                            world: &mut world,
                            res: &mut res,
                            spawner: &mut spawner,
                            loader: &mut loader,
                            scope: &mut scope,
                            control: &mut control,
                            graphics: &mut graphics,
                        },
                        cfg.teardown_timeout.into(),
                    )
                    .await;

                drop(renderer);
                drop(world);
                return Ok(());
            }

            let clock = clocks.advance();
            let mut cx = SystemContext {
                world: &mut world,
                res: &mut res,
                control: &mut control,
                spawner: &mut spawner,
                graphics: &mut graphics,
                loader: &mut loader,
                scope: &mut scope,
                clock,
            };

            scheduler
                .run(cx.reborrow())
                .wrap_err_with(|| "System returned error")?;

            step_ns += clock.delta.as_nanos();

            if step_ns > main_step.as_nanos() {
                step_ns -= main_step.as_nanos();

                #[cfg(feature = "client")]
                if let Some(client) = &mut client {
                    client
                        .run(cx.world, cx.scope)
                        .await
                        .wrap_err("Client system run failed")?;
                }

                #[cfg(feature = "server")]
                if let Some(server) = &mut server {
                    server
                        .run(cx.world, cx.scope)
                        .await
                        .wrap_err("Server system run failed")?;
                }
            }

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
                        spawner: &mut spawner,
                        loader: &loader,
                        scope: &scope,
                        clock,
                        graphics: &mut graphics,
                    },
                    &mut [&mut viewport],
                )
                .wrap_err_with(|| "Renderer failed")?;

            scope.reset();
        }
    });
}

#[cfg(feature = "visible")]
pub fn headless<F, Fut>(_f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    panic!("This function must be used only with \"visible\" feature disabled")
}

#[cfg(not(feature = "visible"))]
pub fn headless<F, Fut>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    crate::install_eyre_handler();
    crate::install_tracing_subscriber();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    // Load config.
    let cfg = load_default_config();

    let teardown_timeout = cfg.teardown_timeout;
    let main_step = cfg.main_step;

    // Initialize asset loader.
    let mut loader_builder = Loader::builder();
    if let Some(path) = cfg.treasury {
        let treasury = goods::source::treasury::TreasurySource::open(path)
            .expect("Failed to initialize treasury loader");
        loader_builder.add(treasury);
    }
    let loader = loader_builder.build();

    // Create new world with camera.
    let world = World::new();

    let spawner = Spawner::new();
    let res = Res::new();

    runtime
        .block_on(async move {
            // Configure game with closure.
            let game = f(Game {
                res,
                world,
                scheduler: Scheduler::with_tick_span(main_step),
                loader,
                spawner,
                scope: Scope::new(),

                #[cfg(feature = "client")]
                client: None,

                #[cfg(feature = "server")]
                server: None,
            })
            .await
            .wrap_err_with(|| "Game startup failed")?;

            let Game {
                mut res,
                mut world,
                mut scheduler,
                mut loader,
                mut spawner,
                mut scope,

                #[cfg(feature = "client")]
                mut client,

                #[cfg(feature = "server")]
                mut server,
            } = game;

            scope.reset();

            // Start the clocks.
            let mut clocks = Clocks::new();
            let mut next = clocks.get_start();

            // Schedule default systems.
            #[cfg(any(feature = "2d", feature = "3d"))]
            scheduler.add_ticking_system(SceneSystem::new());
            scheduler.add_ticking_system(LifeSpanSystem);

            res.insert(FpsMeter::new(TimeSpan::SECOND));
            scheduler.add_fixed_system(
                |cx: SystemContext<'_>| {
                    let fps = cx.res.get::<FpsMeter>().unwrap();
                    tracing::info!("FPS: {}", fps.fps());
                },
                TimeSpan::SECOND,
            );

            let mut executor = Executor::new();

            loop {
                if res.get::<Exit>().is_some() {
                    // Try to finish outstanding async tasks.
                    executor
                        .teardown(
                            TaskContext {
                                world: &mut world,
                                res: &mut res,
                                spawner: &mut spawner,
                                loader: &mut loader,
                                scope: &mut scope,
                            },
                            teardown_timeout.into(),
                        )
                        .await;

                    drop(world);

                    return Ok::<(), eyre::Report>(());
                }

                let clock = clocks.advance();

                let mut cx = SystemContext {
                    world: &mut world,
                    res: &mut res,
                    spawner: &mut spawner,
                    loader: &mut loader,
                    scope: &mut scope,
                    clock,
                };

                scheduler
                    .run(cx.reborrow())
                    .wrap_err_with(|| "System returned error")?;

                #[cfg(feature = "client")]
                if let Some(client) = &mut client {
                    client
                        .run(cx.world, cx.scope)
                        .await
                        .wrap_err("Client system run failed")?;
                }

                #[cfg(feature = "server")]
                if let Some(server) = &mut server {
                    server
                        .run(cx.world, cx.scope)
                        .await
                        .wrap_err("Server system run failed")?;
                }

                executor.append(&mut spawner);
                executor
                    .run_once(TaskContext {
                        world: &mut world,
                        res: &mut res,
                        spawner: &mut spawner,
                        loader: &mut loader,
                        scope: &mut scope,
                    })
                    .wrap_err_with(|| "Task returned error")?;

                res.get_mut::<FpsMeter>()
                    .unwrap()
                    .add_frame_time(clock.delta);

                next += Duration::from(main_step);
                tokio::time::sleep_until(next.into()).await;

                scope.reset();
            }
        })
        .unwrap()
}

#[cfg(feature = "visible")]
struct GameFunnel<'a> {
    viewport: &'a mut Viewport,
    control: &'a mut Control,
}

#[cfg(feature = "visible")]
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
    teardown_timeout: TimeSpan,

    #[serde(default = "default_main_step")]
    main_step: TimeSpan,
}

fn default_teardown_timeout() -> TimeSpan {
    TimeSpan::from_seconds(5)
}

fn default_main_step() -> TimeSpan {
    TimeSpan::from_millis(20)
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
