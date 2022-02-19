use std::future::Future;

#[cfg(feature = "asset-pipeline")]
use std::path::{Path, PathBuf};

use edict::world::World;
use eyre::WrapErr;
use goods::Loader;
use scoped_arena::Scope;

#[cfg(feature = "client")]
use evoke::client::ClientSystem;

#[cfg(feature = "server")]
use evoke::server::ServerSystem;

#[cfg(feature = "visible")]
use winit::window::{Window, WindowBuilder};

use crate::{
    assets::Assets,
    cfg::Config,
    clocks::Clocks,
    lifespan::LifeSpanSystem,
    resources::Res,
    system::{Scheduler, SystemContext},
    task::{Spawner, TaskContext},
};

#[cfg(any(feature = "2d", feature = "3d"))]
use crate::scene::SceneSystem;

#[cfg(feature = "visible")]
use crate::{
    clocks::TimeSpan,
    control::Control,
    edict::bundle::DynamicBundle,
    event::{Event, Loop, WindowEvent},
    fps::FpsMeter,
    funnel::Funnel,
};

#[cfg(feature = "graphics")]
use crate::{
    graphics::{Graphics, Renderer, RendererContext},
    viewport::Viewport,
};

#[cfg(all(any(feature = "2d", feature = "3d"), feature = "graphics"))]
use crate::graphics::renderer::simple::SimpleRenderer;

#[cfg(all(feature = "2d", feature = "graphics"))]
use crate::{camera::Camera2, graphics::renderer::sprite::SpriteDraw, scene::Global2};

#[cfg(all(feature = "3d", feature = "graphics"))]
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
    pub assets: Assets,
    pub spawner: Spawner,
    pub scope: Scope<'static>,

    #[cfg(feature = "visible")]
    pub control: Control,

    #[cfg(feature = "visible")]
    pub funnel: Option<Box<dyn Funnel<Event>>>,

    #[cfg(feature = "graphics")]
    pub graphics: Graphics,

    #[cfg(feature = "graphics")]
    pub renderer: Option<Box<dyn Renderer>>,

    #[cfg(feature = "graphics")]
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
            assets: &mut self.assets,
            scope: &mut self.scope,
            #[cfg(feature = "visible")]
            control: &mut self.control,
            #[cfg(feature = "graphics")]
            graphics: &mut self.graphics,
            #[cfg(not(feature = "graphics"))]
            graphics: Box::leak(Box::new(())),
            #[cfg(feature = "client")]
            client: &mut self.client,
            #[cfg(feature = "server")]
            server: &mut self.server,
        }
    }
}

#[cfg(all(feature = "visible", feature = "graphics", feature = "2d"))]
pub fn game2<F, Fut>(f: F) -> !
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    tracing::debug!("Starting 2D game");
    game::<_, _, _, (Camera2, Global2)>(f, |g| {
        Ok(Box::new(SimpleRenderer::new(SpriteDraw::new(0.0..1.0, g)?)))
    })
}

#[cfg(all(feature = "visible", feature = "graphics", feature = "3d"))]
pub fn game3<F, Fut>(f: F) -> !
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    tracing::debug!("Starting 3D game");
    game::<_, _, _, (Camera3, Global3)>(f, |g| {
        Ok(Box::new(SimpleRenderer::new(BasicDraw::new(g)?)))
    })
}

#[cfg(all(feature = "visible", feature = "graphics"))]
pub fn game<F, Fut, R, C>(f: F, r: R) -> !
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
        let mut cfg = Config::load_default();

        // Initialize asset loader.
        let loader = configure_loader(&mut cfg).await?;

        let assets = Assets::new(loader);

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
            if let Ok(camera) = world.query_one_mut::<&mut Camera2>(&camera) {
                camera.set_aspect(aspect);
            }

            #[cfg(feature = "3d")]
            if let Ok(camera) = world.query_one_mut::<&mut Camera3>(&camera) {
                camera.set_aspect(aspect);
            }
        }

        // Initialize graphics system.
        let graphics = Graphics::new().wrap_err_with(|| "Failed to initialize graphics")?;

        let mut res = Res::new();

        // Attach viewport to window and camera.
        let viewport = Viewport::new(camera, &window, &mut res, &graphics)
            .wrap_err_with(|| "Failed to initialize main viewport")?;

        res.insert(window);

        let spawner = Spawner::new();

        // Configure game with closure.
        let game = f(Game {
            res,
            world,
            scheduler: Scheduler::with_tick_span(cfg.main_step),
            control: Control::new(),
            funnel: None,
            graphics,
            renderer: None,
            viewport,
            assets,
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
            mut funnel,
            mut graphics,
            renderer,
            mut viewport,
            mut assets,
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

        let main_step = cfg.main_step;
        let mut step_ns = 0;

        // Begin game loop.
        loop {
            loop {
                let event = event_loop.next_event(TimeSpan::MILLISECOND).await;

                // Loop through new  events.
                let mut funnel = GameFunnel {
                    viewport: &mut viewport,
                    custom: match &mut funnel {
                        None => None,
                        Some(funnel) => Some(&mut **funnel),
                    },
                    control: &mut control,
                };

                // Filter event
                let event = funnel.filter(&mut res, &mut world, event);

                match event {
                    Some(Event::Loop) => break, // No new events. Continue game loop
                    _ => {}
                }
            }

            if res.get::<Exit>().is_some() {
                // Try to finish outstanding async tasks.
                Spawner::teardown(
                    TaskContext {
                        world: &mut world,
                        res: &mut res,
                        spawner: &mut spawner,
                        assets: &mut assets,
                        scope: &mut scope,
                        control: &mut control,
                        graphics: &mut graphics,

                        #[cfg(feature = "client")]
                        client: &mut client,

                        #[cfg(feature = "server")]
                        server: &mut server,
                    },
                    cfg.teardown_timeout.into(),
                )
                .await;

                drop(renderer);
                drop(world);
                return Ok(());
            }

            Spawner::run_once(TaskContext {
                world: &mut world,
                res: &mut res,
                control: &mut control,
                spawner: &mut spawner,
                graphics: &mut graphics,
                assets: &mut assets,
                scope: &mut scope,

                #[cfg(feature = "client")]
                client: &mut client,

                #[cfg(feature = "server")]
                server: &mut server,
            });

            let clock = clocks.advance();
            let mut cx = SystemContext {
                world: &mut world,
                res: &mut res,
                control: &mut control,
                spawner: &mut spawner,
                graphics: &mut graphics,
                assets: &mut assets,
                scope: &mut scope,
                clock,

                #[cfg(feature = "client")]
                client: &mut client,

                #[cfg(feature = "server")]
                server: &mut server,
            };

            scheduler.run(cx.reborrow());

            step_ns += clock.delta.as_nanos();
            if step_ns > main_step.as_nanos() {
                step_ns %= main_step.as_nanos();

                #[cfg(feature = "client")]
                if let Some(client) = &mut client {
                    client
                        .run(&mut world, &scope)
                        .await
                        .wrap_err("Client system run failed")?;
                }

                #[cfg(feature = "server")]
                if let Some(server) = &mut server {
                    server
                        .run(&mut world, &scope)
                        .await
                        .wrap_err("Server system run failed")?;
                }
            }

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
                        assets: &mut assets,
                        scope: &scope,
                        clock,
                        graphics: &mut graphics,
                    },
                    &mut [&mut viewport],
                )
                .wrap_err_with(|| "Renderer failed")?;

            scope.reset();

            assets.cleanup();
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
    let mut cfg = Config::load_default();

    let teardown_timeout = cfg.teardown_timeout;
    let main_step = cfg.main_step;

    // Create new world with camera.
    let world = World::new();

    let spawner = Spawner::new();
    let res = Res::new();

    runtime
        .block_on(async move {
            // Initialize asset loader.
            let loader = configure_loader(&mut cfg).await?;
            let assets = Assets::new(loader);

            // Configure game with closure.
            let game = f(Game {
                res,
                world,
                scheduler: Scheduler::with_tick_span(main_step),
                assets,
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
                mut assets,
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

            // Schedule default systems.
            #[cfg(any(feature = "2d", feature = "3d"))]
            scheduler.add_ticking_system(SceneSystem::new());

            scheduler.add_ticking_system(LifeSpanSystem);

            loop {
                if res.get::<Exit>().is_some() {
                    // Try to finish outstanding async tasks.
                    Spawner::teardown(
                        TaskContext {
                            world: &mut world,
                            res: &mut res,
                            spawner: &mut spawner,
                            assets: &mut assets,
                            scope: &mut scope,
                            graphics: &mut (),

                            #[cfg(feature = "client")]
                            client: &mut client,

                            #[cfg(feature = "server")]
                            server: &mut server,
                        },
                        teardown_timeout.into(),
                    )
                    .await;

                    drop(world);

                    return Ok::<(), eyre::Report>(());
                }

                Spawner::run_once(TaskContext {
                    world: &mut world,
                    res: &mut res,
                    spawner: &mut spawner,
                    assets: &mut assets,
                    scope: &mut scope,
                    graphics: &mut (),

                    #[cfg(feature = "client")]
                    client: &mut client,

                    #[cfg(feature = "server")]
                    server: &mut server,
                });

                let clock = clocks.advance();

                let mut cx = SystemContext {
                    world: &mut world,
                    res: &mut res,
                    spawner: &mut spawner,
                    assets: &mut assets,
                    scope: &mut scope,
                    clock,
                    graphics: &mut (),

                    #[cfg(feature = "client")]
                    client: &mut client,

                    #[cfg(feature = "server")]
                    server: &mut server,
                };

                scheduler.run(cx.reborrow());

                #[cfg(feature = "client")]
                if let Some(client) = &mut client {
                    client
                        .run(&mut world, &scope)
                        .await
                        .wrap_err("Client system run failed")?;
                }

                #[cfg(feature = "server")]
                if let Some(server) = &mut server {
                    server
                        .run(&mut world, &scope)
                        .await
                        .wrap_err("Server system run failed")?;
                }

                scope.reset();

                tokio::time::sleep_until(
                    clocks
                        .time_stamp_to_instant(scheduler.next_system_run())
                        .into(),
                )
                .await;

                assets.cleanup();
            }
        })
        .unwrap()
}

#[cfg(all(feature = "visible", feature = "graphics"))]
struct GameFunnel<'a> {
    viewport: &'a mut Viewport,
    custom: Option<&'a mut dyn Funnel<Event>>,
    control: &'a mut Control,
}

#[cfg(all(feature = "visible", feature = "graphics"))]
impl Funnel<Event> for GameFunnel<'_> {
    fn filter(&mut self, res: &mut Res, world: &mut World, event: Event) -> Option<Event> {
        let mut event = MainWindowFunnel.filter(res, world, event)?;
        event = self.viewport.filter(res, world, event)?;
        if self.viewport.focused() {
            if let Some(custom) = self.custom.as_deref_mut() {
                event = custom.filter(res, world, event)?;
            }
            event = self.control.filter(res, world, event)?;
        }
        Some(event)
    }
}

#[allow(unused)]
async fn configure_loader(cfg: &Config) -> eyre::Result<Loader> {
    #[allow(unused_mut)]
    let mut loader_builder = Loader::builder();

    #[cfg(feature = "asset-pipeline")]
    if let Some(treasury) = &cfg.treasury {
        match init_treasury(&cfg.root, &treasury) {
            Err(err) => tracing::error!("Failed to initialize treasury loader. {:#}", err),
            Ok(treasury) => {
                tracing::info!("Treasury source configured");
                loader_builder.add(treasury);
            }
        }
    }

    Ok(loader_builder.build())
}

#[cfg(feature = "asset-pipeline")]
fn init_treasury(
    root: &Path,
    cfg: &crate::cfg::TreasuryConfig,
) -> eyre::Result<crate::assets::treasury::TreasurySource> {
    use crate::assets::import::*;

    let base = root.join(&cfg.base);
    let info = treasury_store::TreasuryInfo {
        artifacts: cfg
            .artifacts
            .as_ref()
            .map(|path| change_base(root, &base, path))
            .transpose()?,
        external: cfg
            .external
            .as_ref()
            .map(|path| change_base(root, &base, path))
            .transpose()?,
        temp: cfg
            .temp
            .as_ref()
            .map(|path| change_base(root, &base, path))
            .transpose()?,
        importers: cfg
            .importers
            .iter()
            .map(|path| change_base(root, &base, path))
            .collect::<Result<Vec<_>, _>>()?,
    };

    let mut store = treasury_store::Treasury::new(&base, info)?;

    store.register_importer(ImageImporter);

    // #[cfg(all(feature = "asset-pipeline", feature = "2d"))]
    // store.register_importer(SpriteSheetImporter);
    // store.register_importer(TileMapImporter);
    // store.register_importer(TileSetImporter);

    // #[cfg(all(feature = "asset-pipeline", feature = "3d"))]
    // store.register_importer(GltfModelImporter);

    Ok(crate::assets::treasury::TreasurySource::new(store))
}

#[cfg(feature = "asset-pipeline")]
fn change_base(root: &Path, base: &Path, path: &Path) -> eyre::Result<PathBuf> {
    let path = if path.is_relative() {
        root.join(path)
    } else {
        path.to_owned()
    };

    match path.strip_prefix(base) {
        Ok(path) => Ok(path.to_owned()),
        Err(_) => Ok(path),
    }
}
