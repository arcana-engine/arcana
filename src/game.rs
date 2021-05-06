use {
    crate::{
        assets::Loader,
        camera::{Camera2, Camera3},
        clocks::Clocks,
        control::Control,
        event::{Event, Loop, WindowEvent},
        funnel::Funnel,
        graphics::{
            renderer::{basic::BasicRenderer, sprite::SpriteRenderer},
            Graphics, Renderer, RendererContext,
        },
        prefab::{prefab_pipe, PrefabLoader},
        resources::Res,
        scene::{Global2, Global3, SceneSystem},
        system::{Scheduler, SystemContext},
        viewport::Viewport,
    },
    hecs::World,
    std::{
        collections::VecDeque,
        future::Future,
        time::{Duration, Instant},
    },
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
    pub loader: PrefabLoader,
    pub graphics: Graphics,
    pub renderer: Option<Box<dyn Renderer>>,
    pub viewport: Viewport,
}

pub fn game2<F, Fut>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    crate::install_eyre_handler();
    crate::install_tracing_subscriber();

    Loop::run(|event_loop| async move {
        let (loader, mut spawner) = prefab_pipe(Loader::with_default_sources());

        let mut world = World::new();
        let camera = world.spawn((Camera2::default(), Global2::identity()));

        let mut window = MainWindow::new(&event_loop)?;
        let graphics = Graphics::new()?;
        let viewport = Viewport::new(camera, &window, &graphics)?;

        let game = f(Game {
            res: Res::new(),
            world,
            scheduler: Scheduler::new(),
            control: Control::new(),
            loader,
            graphics,
            renderer: None,
            viewport,
        })
        .await?;

        let Game {
            mut res,
            mut world,
            mut scheduler,
            mut control,
            loader,
            mut graphics,
            renderer,
            mut viewport,
        } = game;

        let mut bump = bumpalo::Bump::new();

        let mut renderer = match renderer {
            Some(renderer) => renderer,
            None => Box::new(SpriteRenderer::new(&mut graphics)?),
        };

        let mut clocks = Clocks::new();

        scheduler.add_system(SceneSystem);
        scheduler.start(clocks.start());

        let mut frames = VecDeque::new();
        let mut last_fps_report = clocks.start();

        loop {
            loop {
                match Funnel::filter(
                    &mut [
                        &mut window as &mut dyn Funnel<Event>,
                        &mut viewport,
                        &mut control,
                    ],
                    &mut res,
                    &mut world,
                    event_loop.next_event(Duration::new(0, 16_666_666)).await,
                ) {
                    Some(Event::Loop) => break,
                    Some(Event::Exit) => {
                        drop(renderer);
                        drop(world);
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

            spawner.flush(&mut res, &mut world, &mut graphics);
            scheduler.run(SystemContext {
                world: &mut world,
                res: &mut res,
                control: &mut control,
                loader: &loader,
                clock,
                bump: &bump,
            })?;

            graphics.flush_uploads(&bump)?;

            renderer.render(
                RendererContext {
                    res: &mut res,
                    world: &mut world,
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

pub fn game3<F, Fut>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    crate::install_eyre_handler();
    crate::install_tracing_subscriber();

    Loop::run(|event_loop| async move {
        let (loader, mut spawner) = prefab_pipe(Loader::with_default_sources());

        let mut world = World::new();
        let camera = world.spawn((Camera3::default(), Global3::identity()));

        let mut window = MainWindow::new(&event_loop)?;
        let graphics = Graphics::new()?;
        let viewport = Viewport::new(camera, &window, &graphics)?;

        let game = f(Game {
            res: Res::new(),
            world,
            scheduler: Scheduler::new(),
            control: Control::new(),
            loader,
            graphics,
            renderer: None,
            viewport,
        })
        .await?;

        let Game {
            mut res,
            mut world,
            mut scheduler,
            mut control,
            loader,
            mut graphics,
            renderer,
            mut viewport,
        } = game;

        let mut bump = bumpalo::Bump::new();

        let mut renderer = match renderer {
            Some(renderer) => renderer,
            None => Box::new(BasicRenderer::new(&mut graphics)?),
        };

        let mut clocks = Clocks::new();

        scheduler.add_system(SceneSystem);
        scheduler.start(clocks.start());

        loop {
            loop {
                match Funnel::filter(
                    &mut [
                        &mut window as &mut dyn Funnel<Event>,
                        &mut viewport,
                        &mut control,
                    ],
                    &mut res,
                    &mut world,
                    event_loop.next_event(Duration::new(0, 16_666_666)).await,
                ) {
                    Some(Event::Loop) => break,
                    Some(Event::Exit) => {
                        drop(renderer);
                        drop(world);
                        graphics.wait_idle();
                        return Ok(());
                    }
                    _ => {}
                }
            }
            let clock = clocks.step();

            spawner.flush(&mut res, &mut world, &mut graphics);
            scheduler.run(SystemContext {
                world: &mut world,
                res: &mut res,
                control: &mut control,
                loader: &loader,
                clock,
                bump: &bump,
            })?;

            graphics.flush_uploads(&bump)?;

            renderer.render(
                RendererContext {
                    res: &mut res,
                    world: &mut world,
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
