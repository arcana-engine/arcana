use {
    crate::{
        assets::Loader,
        camera::Camera3d,
        clocks::Clocks,
        control::Control,
        event::{Event, Loop, WindowEvent},
        funnel::Funnel,
        graphics::{BasicRenderer, Graphics, Renderer, RendererContext},
        prefab::{prefab_pipe, PrefabLoader},
        resources::Res,
        scene::Global3,
        system::{Scheduler, SystemContext},
        viewport::Viewport,
    },
    hecs::World,
    std::{future::Future, time::Duration},
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

// pub struct GameRun<'a> {
//     /// Main world.
//     pub world: &'a mut World,

//     /// Resources map.
//     pub res: &'a mut Res,

//     /// Main camera entity.
//     pub camera: Entity,
// }

// /// Simple application which can be started in few lines of code.
// /// Initializes all default subsystems, allows adding custom systems into ECS
// /// and load first prefab.
// pub struct Game {
//     scheduler: Scheduler,
//     renderer: fn(&mut Graphics) -> eyre::Result<Box<dyn Renderer>>,
//     prefabs: Vec<Box<dyn FnOnce(&PrefabLoader, &Loader, &mut World) + Send + Sync>>,
//     world: World,
//     camera: Entity,
// }

// impl Game {
//     /// Create new instance of [`SimpleApp`].
//     pub fn new() -> Self {
//         let mut world = World::new();
//         let camera = world.spawn((Camera3d::default(), Global3::identity()));

//         Game {
//             scheduler: Scheduler::new(),
//             renderer: |graphics| {
//                 let renderer = BasicRenderer::new(graphics)?;
//                 Ok(Box::new(renderer))
//             },
//             prefabs: Vec::new(),
//             world,
//             camera,
//         }
//     }

//     /// Adds system to the app.
//     pub fn with_system(mut self, system: impl System) -> Self {
//         self.scheduler.add_system(system);
//         self
//     }

//     /// Adds system to the app.
//     pub fn add_system(&mut self, system: impl System) -> &mut Self {
//         self.scheduler.add_system(system);
//         self
//     }

//     /// Adds fixed-step system to the app.
//     pub fn with_fixed_system(mut self, system: impl System, step: Duration) -> Self {
//         self.scheduler.add_fixed_system(system, step);
//         self
//     }

//     /// Adds fixed-step system to the app.
//     pub fn add_fixed_system(&mut self, system: impl System, step: Duration) -> &mut Self {
//         self.scheduler.add_fixed_system(system, step);
//         self
//     }

//     /// Adds fixed-step system to the app.
//     pub fn add_prefab(&mut self, prefab: impl Prefab) -> &mut Self {
//         self.prefabs.push(Box::new(|prefab_loader, loader, world| {
//             prefab_loader.load_prefab(prefab, loader, world);
//         }));
//         self
//     }

//     /// Adds fixed-step system to the app.
//     pub fn with_prefab(mut self, prefab: impl Prefab) -> Self {
//         self.prefabs.push(Box::new(|prefab_loader, loader, world| {
//             prefab_loader.load_prefab(prefab, loader, world);
//         }));
//         self
//     }

//     pub fn with_renderer<R: Renderer>(mut self) -> Self {
//         self.set_renderer::<R>();
//         self
//     }

//     pub fn set_renderer<R: Renderer>(&mut self) -> &mut Self {
//         self.renderer = |graphics| {
//             let renderer = R::new(graphics)?;
//             Ok(Box::new(renderer))
//         };
//         self
//     }

//     /// Runs the app.
//     pub fn run<T, F>(self, control: F) -> !
//     where
//         T: Command,
//         F: FnOnce(GameRun) -> eyre::Result<InputController<T>> + 'static,
//     {
//         let Game {
//             mut scheduler,
//             renderer,
//             prefabs,
//             mut world,
//             camera,
//         } = self;

//         let (prefab_loader, mut prefab_spawner) = prefab_pipe();
//         let loader = Loader::with_default_sources();

//         crate::install_eyre_handler();
//         crate::install_tracing_subscriber();

//         Loop::run(|event_loop| async move {
//             for prefab in prefabs {
//                 prefab(&prefab_loader, &loader, &mut world);
//             }

//             let mut res = Res::new();
//             res.insert(loader);
//             res.insert(prefab_loader);

//             let mut bump = bumpalo::Bump::new();

//             let mut window = MainWindow::new(&event_loop)?;
//             let mut graphics = Graphics::new()?;
//             let mut renderer = renderer(&mut graphics)?;

//             let mut viewport = Viewport::new(camera, &window, &mut graphics)?;

//             let mut clocks = Clocks::new();

//             scheduler.start(clocks.start());

//             let mut control = control(GameRun {
//                 world: &mut world,
//                 res: &mut res,
//                 camera,
//             })?;

//             loop {
//                 loop {
//                     match run_funnel(
//                         &mut [&mut window, &mut viewport, &mut control],
//                         &mut res,
//                         &mut world,
//                         event_loop.next_event(Duration::new(0, 16_666_666)).await,
//                     ) {
//                         Some(Event::Loop) => break,
//                         Some(Event::Exit) => return Ok(()),
//                         _ => {}
//                     }
//                 }
//                 let clock = clocks.step();

//                 prefab_spawner.flush(&mut res, &mut world, &mut graphics);
//                 scheduler.run(SystemContext {
//                     world: &mut world,
//                     res: &mut res,
//                     clock,
//                     bump: &bump,
//                 })?;

//                 graphics.flush_uploads(&bump)?;

//                 renderer.render(
//                     RendererContext {
//                         res: &mut res,
//                         world: &mut world,
//                         graphics: &mut graphics,
//                         bump: &bump,
//                         clock,
//                     },
//                     &mut [&mut viewport],
//                 )?;

//                 bump.reset();
//             }
//         });
//     }
// }

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

pub struct GameCamera;

pub fn game<F, Fut>(f: F)
where
    F: FnOnce(Game) -> Fut + 'static,
    Fut: Future<Output = eyre::Result<Game>>,
{
    crate::install_eyre_handler();
    crate::install_tracing_subscriber();

    Loop::run(|event_loop| async move {
        let (loader, mut spawner) = prefab_pipe(Loader::with_default_sources());

        let mut world = World::new();
        let camera = world.spawn((Camera3d::default(), Global3::identity()));

        let mut window = MainWindow::new(&event_loop)?;
        let graphics = Graphics::new()?;
        let viewport = Viewport::new(camera, &window, &graphics)?;

        let mut game = f(Game {
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
                    Some(Event::Exit) => return Ok(()),
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
