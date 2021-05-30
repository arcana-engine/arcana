use hecs::DynamicBundle;

use {
    crate::{
        camera::{Camera2, Camera3},
        clocks::Clocks,
        control::Control,
        event::{Event, Loop, WindowEvent},
        funnel::Funnel,
        graphics::{
            renderer::{basic::BasicRenderer, sprite::SpriteRenderer},
            Graphics, Renderer, RendererContext,
        },
        resources::Res,
        scene::{Global2, Global3, SceneSystem},
        system::{Scheduler, SystemContext},
        task::{Executor, Spawner, TaskContext},
        viewport::Viewport,
    },
    goods::Loader,
    hecs::World,
    std::{collections::VecDeque, future::Future, path::Path, time::Duration},
    winit::window::Window,
};

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
            window: Window::new(event_loop)?,
        })
    }
}

impl Funnel<Event> for MainWindow {
    fn filter(&mut self, _res: &mut Res, _world: &mut World, event: Event) -> Option<Event> {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == self.window.id() => Some(Event::Exit),
            Event::Loop => {
                self.window.request_redraw();
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
}

pub fn game2<F, Fut>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    game::<_, _, SpriteRenderer, (Camera2, Global2)>(f)
}

pub fn game3<F, Fut>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    game::<_, _, BasicRenderer, (Camera3, Global3)>(f)
}

fn game<F, Fut, R, C>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
    R: Renderer + Send,
    C: DynamicBundle + Default,
{
    crate::install_eyre_handler();
    crate::install_tracing_subscriber();

    Loop::run(|event_loop| async move {
        // Load config.
        let cfg = load_default_config()?;

        // Initialize asset loader.
        let mut loader_builder = Loader::builder();
        if let Some(path) = cfg.treasury {
            let treasury = goods::source::treasury::TreasurySource::open(path)?;
            loader_builder.add(treasury);
        }
        let loader = loader_builder.build();

        // Create new world with camera.
        let mut world = World::new();
        let camera = world.spawn(C::default());

        // Open game window.
        let mut window = MainWindow::new(&event_loop)?;

        // Initialize graphics system.
        let graphics = Graphics::new()?;

        // Attach viewport to window and camera.
        let viewport = Viewport::new(camera, &window, &graphics)?;
        let spawner = Spawner::new();

        // Configure game with closure.
        let game = f(Game {
            res: Res::new(),
            world,
            scheduler: Scheduler::new(),
            control: Control::new(),
            graphics,
            renderer: None,
            viewport,
            loader,
            spawner,
        })
        .await?;

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
        } = game;

        // Take renderer. Use default one if not configured.
        let mut renderer = match renderer {
            Some(renderer) => renderer,
            None => Box::new(R::new(&mut graphics)?),
        };

        // Start the clocks.
        let mut clocks = Clocks::new();

        // Schedule default systems.
        scheduler.add_system(SceneSystem);

        scheduler.start(clocks.start());

        let mut executor = Executor::new();

        let mut frames = VecDeque::new();
        let mut last_fps_report = clocks.start();

        // Init bumpalo allocator.
        let mut bump = bumpalo::Bump::new();

        // Begin game loop.
        loop {
            // Loop through new  events.
            let mut funnel = GameFunnel {
                window: &mut window,
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
                                    bump: &bump,
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

            let clock = clocks.step();

            frames.push_back(clock.current);

            while let Some(frame) = frames.pop_front() {
                if frame + Duration::from_secs(5) > clock.current || frames.len() < 300 {
                    frames.push_front(frame);
                    break;
                }
            }

            if last_fps_report + Duration::from_secs(1) < clock.current && frames.len() > 10 {
                last_fps_report = clock.current;
                let window = (*frames.back().unwrap() - *frames.front().unwrap()).as_secs_f32();
                let fps = frames.len() as f32 / window;
                tracing::info!("FPS: {}", fps);
            }

            scheduler.run(SystemContext {
                world: &mut world,
                res: &mut res,
                control: &mut control,
                spawner: &mut spawner,
                graphics: &mut graphics,
                loader: &mut loader,
                bump: &bump,
                clock,
            })?;

            executor.append(&mut spawner);
            executor.run_once(TaskContext {
                world: &mut world,
                res: &mut res,
                control: &mut control,
                spawner: &mut spawner,
                graphics: &mut graphics,
                loader: &mut loader,
                bump: &bump,
            })?;

            graphics.flush_uploads(&bump)?;

            renderer.render(
                RendererContext {
                    world: &mut world,
                    res: &mut res,
                    graphics: &mut graphics,
                    bump: &bump,
                    clock,
                },
                &mut [&mut viewport],
            )?;

            bump.reset();
        }
    });
}

struct GameFunnel<'a> {
    window: &'a mut MainWindow,
    viewport: &'a mut Viewport,
    control: &'a mut Control,
}

impl Funnel<Event> for GameFunnel<'_> {
    fn filter(&mut self, res: &mut Res, world: &mut World, event: Event) -> Option<Event> {
        match Funnel::filter(&mut *self.window, res, world, event) {
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

#[derive(serde::Deserialize)]
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

fn load_default_config() -> eyre::Result<Config> {
    let path = Path::new("cfg.json");
    if path.is_file() {
        load_config(path)
    } else {
        let mut path = std::env::current_exe()?;
        path.set_file_name("cfg.json");
        load_config(&path)
    }
}
