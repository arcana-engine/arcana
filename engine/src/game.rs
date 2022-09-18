use std::future::Future;

#[cfg(feature = "asset-pipeline")]
use std::path::Path;

use edict::{scheduler::Scheduler, system::Res, world::World, EntityId};
use eyre::WrapErr;
use goods::Loader;

#[cfg(feature = "visible")]
use winit::{
    dpi::PhysicalSize,
    window::{self, WindowBuilder},
};

use crate::{assets::Assets, cfg::Config, clocks::Clocks, control::ControlFunnel, window::Windows};

#[cfg(feature = "2d")]
use crate::scene::scene_system2;

#[cfg(feature = "3d")]
use crate::scene::scene_system3;

#[cfg(feature = "visible")]
use crate::{
    clocks::TimeSpan,
    control::Control,
    edict::bundle::DynamicComponentBundle,
    event::{Event, Loop, WindowEvent},
    fps::FpsMeter,
    funnel::Funnel,
    lifespan::lifetime_system,
    system::ToFixSystem,
    task::teardown_tasks,
};

#[cfg(feature = "graphics")]
use crate::graphics::{renderer::Renderer, Graphics};

// #[cfg(all(any(feature = "2d", feature = "3d"), feature = "graphics"))]
// use crate::graphics::renderer::simple::SimpleRenderer;

// #[cfg(all(feature = "2d", feature = "graphics"))]
// use crate::{camera::Camera2, graphics::renderer::sprite::SpriteDraw, scene::Global2};

// #[cfg(all(feature = "3d", feature = "graphics"))]
// use crate::{camera::Camera3, graphics::renderer::basic::BasicDraw, scene::Global3};

#[cfg(feature = "visible")]
#[repr(transparent)]
pub struct MainWindow {
    window: window::Window,
}

#[cfg(feature = "visible")]
impl std::ops::Deref for MainWindow {
    type Target = window::Window;

    fn deref(&self) -> &window::Window {
        &self.window
    }
}

#[cfg(feature = "visible")]
impl MainWindow {
    fn new(event_loop: &Loop, size: Option<PhysicalSize<u32>>) -> eyre::Result<Self> {
        let mut builder = WindowBuilder::new().with_title("Arcana Game");

        if let Some(size) = size {
            builder = builder.with_inner_size(size);
        }

        Ok(MainWindow {
            window: builder.build(event_loop)?,
        })
    }
}

#[cfg(feature = "visible")]
struct MainWindowFunnel;

#[cfg(feature = "visible")]
impl Funnel<Event> for MainWindowFunnel {
    fn filter(&mut self, world: &mut World, event: Event) -> Option<Event> {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } => {
                let is_main = world
                    .get_resource::<MainWindow>()
                    .map_or(false, |window| window_id == window.id());

                if is_main {
                    world.insert_resource(Exit);
                    world.remove_resource::<MainWindow>();
                }
            }
            _ => {}
        }
        Some(event)
    }
}

/// Resource that when added exists game loop.
pub struct Exit;

pub struct Game {
    pub world: World,
    pub scheduler: Scheduler,

    #[cfg(feature = "visible")]
    pub funnel: Option<Box<dyn Funnel<Event>>>,

    #[cfg(feature = "graphics")]
    pub renderer: Option<Box<dyn Renderer>>,

    #[cfg(feature = "visible")]
    pub camera: EntityId,
}

#[cfg(all(feature = "visible", feature = "graphics", feature = "2d"))]
pub fn game2<F, Fut>(f: F) -> !
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    tracing::debug!("Starting 2D game");
    // game::<_, _, _, (Camera2, Global2)>(f, |g| {
    //     Ok(Box::new(SimpleRenderer::new(SpriteDraw::new(0.0..1.0, g)?)))
    // })
    todo!()
}

#[cfg(all(feature = "visible", feature = "graphics", feature = "3d"))]
pub fn game3<F, Fut>(f: F) -> !
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    tracing::debug!("Starting 3D game");
    // game::<_, _, _, (Camera3, Global3)>(f, |g| {
    //     Ok(Box::new(SimpleRenderer::new(BasicDraw::new(g)?)))
    // })
    todo!()
}

#[cfg(all(feature = "visible", feature = "graphics"))]
pub fn game<F, Fut, R, C>(f: F, r: R) -> !
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
    R: FnOnce(&mut Graphics) -> eyre::Result<Box<dyn Renderer>> + Send + 'static,
    C: DynamicComponentBundle + Default,
{
    use crate::graphics::spawn_window_render_target;

    crate::install_eyre_handler();
    crate::install_tracing_subscriber();

    Loop::run(|event_loop| async move {
        // Load config.
        let cfg = Config::load_default();

        // Create new world with camera.
        let mut world = World::new();

        // Initialize asset loader.
        let loader = configure_loader(&cfg).await?;
        world.insert_resource(Assets::new(loader));

        // Open game window.
        let window = MainWindow::new(&event_loop, cfg.game.window_size)
            .wrap_err_with(|| "Failed to initialize main window")?;

        let mut windows = Windows::new();

        spawn_window_render_target(&window, &mut world, &mut windows)
            .wrap_err_with(|| "Failed to initialize main window render target")?;

        let camera = world.spawn(C::default());

        // Initialize graphics system.
        let graphics = Graphics::new().wrap_err_with(|| "Failed to initialize graphics")?;
        world.insert_resource(graphics);

        world.insert_resource(Control::new());

        // Configure the game with user-provided closure.
        let game = f(Game {
            world,
            scheduler: Scheduler::new(),
            funnel: None,
            renderer: None,
            camera,
        })
        .await
        .wrap_err_with(|| "Game startup failed")?;

        let Game {
            mut world,
            mut scheduler,
            mut funnel,
            renderer,
            ..
        } = game;

        // Take renderer. Use default one if not configured.
        let renderer = match renderer {
            Some(renderer) => renderer,
            None => {
                let mut graphics = world.expect_resource_mut();
                r(&mut graphics).wrap_err_with(|| "Renderer build failed")?
            }
        };

        // Start the clocks.
        let mut clocks = Clocks::new();

        scheduler.add_system(lifetime_system);

        // Schedule default systems.
        #[cfg(feature = "2d")]
        scheduler.add_system(scene_system2);

        #[cfg(feature = "3d")]
        scheduler.add_system(scene_system3);

        world.insert_resource(FpsMeter::new(TimeSpan::SECOND));
        scheduler.add_system(
            (move |fps: Res<FpsMeter>| {
                tracing::info!("FPS: {}", fps.fps());
            })
            .to_fix_system(TimeSpan::SECOND),
        );

        // Begin game loop.
        loop {
            loop {
                let event = event_loop.next_event(TimeSpan::MILLISECOND).await;

                // Loop through new  events.
                let mut funnel = GameFunnel {
                    windows: &mut windows,
                    custom: match &mut funnel {
                        None => None,
                        Some(funnel) => Some(&mut **funnel),
                    },
                    control: &mut ControlFunnel,
                };

                // Filter event
                let event = funnel.filter(&mut world, event);

                if let Some(Event::Loop) = event {
                    teardown_tasks(&mut world).await;

                    break; // No new events. Continue game loop
                }
            }

            if world.get_resource::<Exit>().is_some() {
                drop(renderer);
                drop(world);
                return Ok(());
            }

            let clock = clocks.advance();

            scheduler.run_rayon(&mut world);

            world
                .expect_resource_mut::<FpsMeter>()
                .add_frame_time(clock.delta);

            world.expect_resource_mut::<Assets>().cleanup();
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
    let cfg = Config::load_default();

    let teardown_timeout = cfg.teardown_timeout;
    let main_step = cfg.main_step;

    // Create new world with camera.
    let world = World::new();

    let spawner = Spawner::new();
    let res = Res::new();

    runtime
        .block_on(async move {
            // Initialize asset loader.
            let loader = configure_loader(&cfg).await?;
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
                world.maintain();
            }
        })
        .unwrap()
}

#[cfg(all(feature = "visible", feature = "graphics"))]
struct GameFunnel<'a> {
    windows: &'a mut Windows,
    custom: Option<&'a mut dyn Funnel<Event>>,
    control: &'a mut ControlFunnel,
}

#[cfg(all(feature = "visible", feature = "graphics"))]
impl Funnel<Event> for GameFunnel<'_> {
    fn filter(&mut self, world: &mut World, event: Event) -> Option<Event> {
        let mut event = MainWindowFunnel.filter(world, event)?;
        event = self.windows.filter(world, event)?;

        if !self.windows.skip_event(&event, world) {
            if let Some(custom) = &mut self.custom {
                event = custom.filter(world, event)?;
            }
            event = self.control.filter(world, event)?;
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
        match init_treasury(&cfg.root, treasury) {
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
        artifacts: cfg.artifacts.as_ref().map(|path| root.join(path)),
        external: cfg.external.as_ref().map(|path| root.join(path)),
        temp: cfg.temp.as_ref().map(|path| root.join(path)),
        importers: cfg.importers.iter().map(|path| root.join(path)).collect(),
    };

    let mut store = treasury_store::Treasury::new(&base, info)?;

    store.register_importer(ImageImporter);

    #[cfg(feature = "asset-pipeline")]
    {
        #[cfg(feature = "2d")]
        {
            #[cfg(feature = "graphics")]
            store.register_importer(SpriteSheetImporter);

            store.register_importer(TileMapImporter);
            store.register_importer(TileSetImporter);
        }

        #[cfg(all(feature = "graphics", feature = "3d"))]
        store.register_importer(GltfModelImporter);
    }

    Ok(crate::assets::treasury::TreasurySource::new(store))
}
