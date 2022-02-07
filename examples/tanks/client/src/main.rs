#![feature(allocator_api, future_poll_fn)]
// #![cfg_attr(windows, windows_subsystem = "windows")]

use std::net::Ipv4Addr;

use arcana::{
    edict::entity::EntityId,
    evoke,
    graphics::{simple::SimpleRenderer, sprite::SpriteDraw, DrawNode, Material},
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
    forward_pressed: bool,
    backward_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    fire_pressed: bool,
}

impl TankCommander {
    pub fn main() -> Self {
        TankCommander {
            forward_pressed: false,
            backward_pressed: false,
            left_pressed: false,
            right_pressed: false,
            fire_pressed: false,
        }
    }
}

impl EventTranslator for TankCommander {
    type Command = TankCommand;

    fn translate(&mut self, event: InputEvent) -> Option<TankCommand> {
        match event {
            InputEvent::KeyboardInput(KeyboardInput {
                state,
                virtual_keycode: Some(key),
                ..
            }) => match key {
                VirtualKeyCode::W => match state {
                    ElementState::Pressed if !self.forward_pressed => {
                        self.forward_pressed = true;
                        Some(TankCommand::Drive(1))
                    }
                    ElementState::Released if self.forward_pressed => {
                        self.forward_pressed = false;
                        Some(TankCommand::Drive(-1))
                    }
                    _ => None,
                },
                VirtualKeyCode::S => match state {
                    ElementState::Pressed if !self.backward_pressed => {
                        self.backward_pressed = true;
                        Some(TankCommand::Drive(-1))
                    }
                    ElementState::Released if self.backward_pressed => {
                        self.backward_pressed = false;
                        Some(TankCommand::Drive(1))
                    }
                    _ => None,
                },
                VirtualKeyCode::A => match state {
                    ElementState::Pressed if !self.left_pressed => {
                        self.left_pressed = true;
                        Some(TankCommand::Rotate(-1))
                    }
                    ElementState::Released if self.left_pressed => {
                        self.left_pressed = false;
                        Some(TankCommand::Rotate(1))
                    }
                    _ => None,
                },
                VirtualKeyCode::D => match state {
                    ElementState::Pressed if !self.right_pressed => {
                        self.right_pressed = true;
                        Some(TankCommand::Rotate(1))
                    }
                    ElementState::Released if self.right_pressed => {
                        self.right_pressed = false;
                        Some(TankCommand::Rotate(-1))
                    }
                    _ => None,
                },
                VirtualKeyCode::Space => match state {
                    ElementState::Pressed if !self.fire_pressed => {
                        self.fire_pressed = true;
                        Some(TankCommand::Fire)
                    }
                    ElementState::Released if self.fire_pressed => {
                        self.fire_pressed = false;
                        None
                    }
                    _ => None,
                },
                _ => None,
            },
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
            if tank.alive {
                if tank.fire {
                    let pos = global.iso.transform_point(&na::Point2::new(0.0, -0.6));
                    let dir = global.iso.transform_vector(&na::Vector2::new(0.0, -10.0));
                    bullets.push((pos, dir));
                    tank.fire = false;
                }
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

fn main() {
    game2(|mut game| async move {
        tracing::info!("START");

        // let ui = game.res.with(SigilsUI::new);

        // ui.enable_text_rendering(
        //     "7f0f3e88-f2b6-4969-b6e6-4f4cd3b6db54".parse().unwrap(),
        //     sigils::Vector2 { x: 256, y: 256 },
        // );

        // let title = ui.add_node(
        //     sigils::Node::new()
        //         .with_anchor(sigils::Anchor::Top)
        //         .with_width_percents(40.0)
        //         .with_height_pixels(50.0)
        //         .with_color([0.294, 0.325, 0.125, 1.0]),
        // );

        // ui.add_text_node(
        //     sigils::Node::new()
        //         .with_parent(title)
        //         .with_color([0.1, 0.1, 0.2, 1.0])
        //         .text("HELLO TANKS!".into()), // .with_fitting(TextFitting::Stretch),
        // );

        // let button = ui.add_textured_node(
        //     sigils::Node::new()
        //         .with_width_percents(30.0)
        //         .with_height_percents(10.0)
        //         .with_padding(6.0, 6.0, 6.0, 6.0)
        //         .textured("c6147dfa-2ea3-43bb-9bda-ccc4350bbe37".parse().unwrap())
        //         .with_slice9(16.0, 16.0, 16.0, 16.0)
        //         .with_slice9_uv(0.4921875, 0.4921875, 0.4921875, 0.4921875),
        // );

        // ui.add_text_node(
        //     sigils::Node::new()
        //         .with_parent(button)
        //         .with_color([0.5, 0.1, 0.6, 1.0])
        //         .text("BUTTON".into()), // .with_fitting(TextFitting::Stretch),
        // );

        // ui.set_extent(sigils::Vector2 {
        //     x: game.viewport.size().width as f32,
        //     y: game.viewport.size().height as f32,
        // });

        let renderer = SimpleRenderer::with_multiple(vec![
            Box::new(SpriteDraw::new(0.0..0.99, &mut game.graphics)?) as Box<dyn DrawNode>,
            // Box::new(SigilsDraw::new(&mut game.graphics)?) as Box<dyn DrawNode>,
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

        // Create client system to communicate with game server.
        let mut client = evoke::client::ClientSystem::builder()
            .with_descriptor::<Tank>()
            .with_descriptor::<TankState>()
            .with_descriptor::<TileMap>()
            .with_descriptor::<Global2>()
            .with_player::<LocalTankPlayer>()
            .build();

        tracing::info!("Connecting to server");

        // Connect to local server. It must be running.
        client
            // .connect((Ipv4Addr::new(62, 84, 122, 89), 12345), &game.scope)
            .connect((Ipv4Addr::LOCALHOST, 12453), &game.scope)
            .await?;

        tracing::info!("Connected");

        // Add player to game session.
        let pid = client.add_player(&(), &game.scope).await?;

        tracing::info!("Player added");

        // Set client session to be executed in game loop.
        game.client = Some(client);

        struct RemoteControl {
            entity: Option<EntityId>,
            pid: evoke::PlayerId,
        }

        let mut rc = RemoteControl { entity: None, pid };

        // Add system that will assume control of entities belonging to the added player.
        game.scheduler.add_system(move |cx: SystemContext<'_>| {
            if let Some(entity) = rc.entity {
                if cx.world.query_one_mut::<&evoke::PlayerId>(&entity).is_err() {
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

                        let controller =
                            EntityController::assume_control(TankCommander::main(), e, cx.world)
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
        });

        // Game configured. Run it.
        Ok(game)
    })
}
