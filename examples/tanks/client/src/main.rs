#![feature(allocator_api, future_poll_fn)]
// #![cfg_attr(windows, windows_subsystem = "windows")]

use std::net::Ipv4Addr;

use arcana::{
    edict::entity::EntityId,
    egui::{self, EguiDraw, EguiFunnel, EguiResource},
    evoke,
    graphics::{
        renderer::{simple::SimpleRenderer, sprite::SpriteDraw, DrawNode},
        Material,
    },
    na,
    physics2::*,
    prelude::*,
    rapier2d::prelude::RigidBodyBuilder,
    rect::Rect,
    scoped_arena::Scope,
    sprite::Sprite,
    tiles::TileMap,
};

use tanks::*;

#[derive(Debug)]
struct TankCommander {
    binder: SimpleKeyBinder<TankCommand>,
}

impl TankCommander {
    pub fn main() -> Self {
        let binder = SimpleKeyBinder::builder()
            .on_switch_with(VirtualKeyCode::W, TankCommand::Drive, 1, -1)
            .on_switch_with(VirtualKeyCode::S, TankCommand::Drive, -1, 1)
            .on_switch_with(VirtualKeyCode::A, TankCommand::Rotate, -1, 1)
            .on_switch_with(VirtualKeyCode::D, TankCommand::Rotate, 1, -1)
            .on_press(VirtualKeyCode::Space, TankCommand::Fire)
            .build();
        TankCommander { binder }
    }
}

impl EventTranslator for TankCommander {
    type Command = TankCommand;

    fn translate(&mut self, event: InputEvent) -> Option<TankCommand> {
        match event {
            InputEvent::KeyboardInput(input) => self.binder.handle_input(&input).copied(),
            _ => None,
        }
    }
}

pub struct TankClientSystem;

impl System for TankClientSystem {
    fn name(&self) -> &str {
        "TankClientSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        let mut bullets = Vec::new_in(&*cx.scope);

        for (_entity, (global, tank)) in cx
            .world
            .query_mut::<(&Global2, &mut TankState)>()
            .with::<Tank>()
        {
            if tank.alive && tank.fire {
                let pos = global.iso.transform_point(&na::Point2::new(0.0, -0.6));
                let dir = global.iso.transform_vector(&na::Vector2::new(0.0, -10.0));
                bullets.push((pos, dir));
                tank.fire = false;
            }
        }

        if !bullets.is_empty() {
            let collider = cx.res.with(BulletCollider::new).0.clone();
            let physics = cx.res.with(PhysicsData2::new);

            for (pos, dir) in bullets {
                let body = physics
                    .bodies
                    .insert(RigidBodyBuilder::new_dynamic().linvel(dir).build());
                physics
                    .colliders
                    .insert_with_parent(collider.clone(), body, &mut physics.bodies);

                cx.world.spawn((
                    Global2::new(na::Translation2::new(pos.x, pos.y).into()),
                    Bullet,
                    body,
                    Sprite {
                        world: Rect {
                            left: -0.05,
                            right: 0.05,
                            top: -0.05,
                            bottom: 0.05,
                        },
                        src: Rect::ONE_QUAD,
                        tex: Rect::ONE_QUAD,
                        layer: 0,
                    },
                    Material {
                        albedo_factor: [1.0, 0.8, 0.2, 1.0],
                        ..Default::default()
                    },
                    ContactQueue2::new(),
                    LifeSpan::new(TimeSpan::SECOND * 3),
                ));
            }
        }
    }
}

enum LocalTankPlayer {}

impl<'a> evoke::client::LocalPlayerPack<'a> for LocalTankPlayer {
    type Pack = &'a [TankCommand];
}

impl evoke::client::LocalPlayer for LocalTankPlayer {
    type Query = &'static mut CommandQueue<TankCommand>;

    fn replicate<'a>(
        queue: &mut CommandQueue<TankCommand>,
        scope: &'a Scope<'_>,
    ) -> &'a [TankCommand] {
        scope.to_scope_from_iter(queue.drain())
    }
}

struct TankConnectionUI {
    error: Option<String>,
    name: String,
    connecting: bool,
    hint_name_nonempty: bool,
}

impl TankConnectionUI {
    pub fn new() -> Self {
        TankConnectionUI {
            error: None,
            name: String::new(),
            connecting: false,
            hint_name_nonempty: false,
        }
    }
}

// struct BulletUI {
//     rgb: [u8; 3],
// }

// impl BulletUI {
//     pub fn new() -> Self {
//         BulletUI {
//             error: None,
//             name: String::new(),
//             connecting: false,
//             hint_name_nonempty: false,
//         }
//     }
// }

struct RemoteControl {
    entity: Option<EntityId>,
    pid: evoke::PlayerId,
}

fn main() {
    game2(|mut game| async move {
        let renderer = SimpleRenderer::with_multiple(vec![
            Box::new(SpriteDraw::new(0.0..0.99, &mut game.graphics)?) as Box<dyn DrawNode>,
            // Box::new(SigilsDraw::new(&mut game.graphics)?) as Box<dyn DrawNode>,
            Box::new(EguiDraw::new(&mut game.graphics)?) as Box<dyn DrawNode>,
        ]);
        game.renderer = Some(Box::new(renderer));

        // Setup camera.
        let camera = game.viewport.camera();

        game.world
            .query_one_mut::<&mut Camera2>(&camera)
            .unwrap()
            .set_scaley(0.2);

        game.scheduler.add_system(tanks::TankAnimationSystem::new());

        game.scheduler
            .add_fixed_system(Physics2::new(), TimeSpan::MILLISECOND * 20);

        Tank::schedule_unfold_system(&mut game.scheduler);
        TileMap::schedule_unfold_system(&mut game.scheduler);
        game.scheduler.add_system(TankClientSystem);
        game.scheduler.add_system(tanks::BulletSystem);

        // Add system that will assume control of entities belonging to the added player.
        game.scheduler.add_system(move |cx: SystemContext<'_>| {
            if let Some(rc) = cx.res.get_mut::<RemoteControl>() {
                if let Some(entity) = rc.entity {
                    if let Ok(false) | Err(_) = cx.world.has_component::<evoke::PlayerId>(&entity) {
                        tracing::error!("Controlled entity is broken");
                        let _ = cx.world.despawn(&entity);
                        rc.entity = None;
                    }
                }

                if rc.entity.is_none() {
                    for (e, pid) in cx
                        .world
                        .query_mut::<&evoke::PlayerId>()
                        .without::<Controlled>()
                    {
                        if rc.pid == *pid {
                            tracing::info!("Found player's entity");

                            let controller = EntityController::assume_control(
                                TankCommander::main(),
                                e,
                                cx.world,
                            )
                            .expect("EntityId exists and is not controlled");

                            cx.control.add_global_controller(controller);
                            rc.entity = Some(e);

                            break;
                        }
                    }
                }

                if let Some(entity) = rc.entity {
                    if let Some(pos) = cx.world.query_one_mut::<&Global2>(&entity).ok().copied() {
                        if let Ok(cam) = cx.world.query_one_mut::<&mut Global2>(&camera) {
                            cam.iso.translation = pos.iso.translation;
                        }
                    }
                }
            }
        });

        let window = game
            .res
            .get::<MainWindow>()
            .expect("Window must be created");
        let egui = EguiResource::new(window);
        game.res.insert(egui);
        game.res.insert(TankConnectionUI::new());

        // Add GUI system
        game.scheduler.add_system(move |cx: SystemContext<'_>| {
            let (egui, window, model) = cx.res.query::<(
                &mut EguiResource,
                &MainWindow,
                Option<&mut TankConnectionUI>,
            )>();

            egui.run(window, |ctx| {
                if let Some(model) = model {
                    egui::Window::new("Login")
                        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                        .enabled(!model.connecting && model.error.is_none())
                        .show(ctx, |ui| {
                            ui.add(egui::Label::new("Connect to server!"));
                            ui.add(egui::TextEdit::singleline(&mut model.name));

                            if model.hint_name_nonempty {
                                ui.add(egui::Label::new("Name must be non-empty"));
                            }

                            if ui.add(egui::Button::new("Connect")).clicked() {
                                if model.name.is_empty() {
                                    model.hint_name_nonempty = true;
                                } else {
                                    model.hint_name_nonempty = false;
                                    model.connecting = true;

                                    cx.spawner.spawn(async move {
                                        // Connecting to the game server.
                                        let scope = Scope::new();

                                        // Create client system to communicate with the game server.
                                        let mut client = evoke::client::ClientSystem::builder()
                                            .with_descriptor::<Tank>()
                                            .with_descriptor::<TankState>()
                                            .with_descriptor::<TileMap>()
                                            .with_descriptor::<Global2>()
                                            .with_player::<LocalTankPlayer>()
                                            .build();

                                        let res = async move {
                                            client
                                                // .connect((Ipv4Addr::new(62, 84, 122, 89), 12345), &game.scope)
                                                .connect((Ipv4Addr::LOCALHOST, 12453), &scope)
                                                .await?;

                                            // Add player to game session.
                                            let pid = client.add_player(&(), &scope).await?;

                                            Ok::<_, eyre::Report>((client, pid))
                                        }
                                        .await;

                                        with_async_task_context(move |cx| match res {
                                            Ok((client, pid)) => {
                                                cx.res.insert(RemoteControl { entity: None, pid });
                                                *cx.client = Some(client);
                                                cx.res.remove::<TankConnectionUI>();
                                            }
                                            Err(err) => {
                                                if let Some(model) =
                                                    cx.res.get_mut::<TankConnectionUI>()
                                                {
                                                    model.connecting = false;
                                                    model.error = Some(err.to_string());
                                                }
                                            }
                                        });
                                    })
                                }
                            }
                        });

                    if model.error.is_some() {
                        egui::Window::new("Error")
                            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                            .show(ctx, |ui| {
                                ui.add(egui::Label::new(model.error.as_deref().unwrap()));
                                if ui.add(egui::Button::new("OK")).clicked() {
                                    model.error = None;
                                }
                            });
                    }
                }
            });
        });

        game.funnel = Some(Box::new(EguiFunnel));

        // Game configured. Run it.
        Ok(game)
    })
}
