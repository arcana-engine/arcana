#![feature(allocator_api, future_poll_fn)]

use std::net::Ipv4Addr;

use arcana::{
    assets::tiles::TileMapComponent,
    camera::Camera2,
    event::{ElementState, KeyboardInput, VirtualKeyCode},
    game2,
    net::client::{ClientSystem, SerdeInputsReplicate},
    net::{PlayerId, ReplicaPrefabSerde},
    Controlled, EntityController, Global2, InputCommander, InputEvent, SystemContext,
};

use tanks::*;

#[derive(Debug)]
pub struct TankComander {
    forward: VirtualKeyCode,
    backward: VirtualKeyCode,
    left: VirtualKeyCode,
    right: VirtualKeyCode,
    fire: VirtualKeyCode,

    forward_pressed: bool,
    backward_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    fire_pressed: bool,
}

impl TankComander {
    pub fn main() -> Self {
        TankComander {
            forward: VirtualKeyCode::W,
            backward: VirtualKeyCode::S,
            left: VirtualKeyCode::A,
            right: VirtualKeyCode::D,
            fire: VirtualKeyCode::Space,

            forward_pressed: false,
            backward_pressed: false,
            left_pressed: false,
            right_pressed: false,
            fire_pressed: false,
        }
    }

    pub fn alt() -> Self {
        TankComander {
            forward: VirtualKeyCode::Up,
            backward: VirtualKeyCode::Down,
            left: VirtualKeyCode::Left,
            right: VirtualKeyCode::Right,
            fire: VirtualKeyCode::Numpad0,

            forward_pressed: false,
            backward_pressed: false,
            left_pressed: false,
            right_pressed: false,
            fire_pressed: false,
        }
    }
}

impl InputCommander for TankComander {
    type Command = TankCommand;

    fn translate(&mut self, event: InputEvent) -> Option<TankCommand> {
        match event {
            InputEvent::KeyboardInput(KeyboardInput {
                state,
                virtual_keycode: Some(key),
                ..
            }) => {
                if key == self.forward {
                    match state {
                        ElementState::Pressed if !self.forward_pressed => {
                            self.forward_pressed = true;
                            Some(TankCommand::Drive(1))
                        }
                        ElementState::Released if self.forward_pressed => {
                            self.forward_pressed = false;
                            Some(TankCommand::Drive(-1))
                        }
                        _ => None,
                    }
                } else if key == self.backward {
                    match state {
                        ElementState::Pressed if !self.backward_pressed => {
                            self.backward_pressed = true;
                            Some(TankCommand::Drive(-1))
                        }
                        ElementState::Released if self.backward_pressed => {
                            self.backward_pressed = false;
                            Some(TankCommand::Drive(1))
                        }
                        _ => None,
                    }
                } else if key == self.left {
                    match state {
                        ElementState::Pressed if !self.left_pressed => {
                            self.left_pressed = true;
                            Some(TankCommand::Rotate(-1))
                        }
                        ElementState::Released if self.left_pressed => {
                            self.left_pressed = false;
                            Some(TankCommand::Rotate(1))
                        }
                        _ => None,
                    }
                } else if key == self.right {
                    match state {
                        ElementState::Pressed if !self.right_pressed => {
                            self.right_pressed = true;
                            Some(TankCommand::Rotate(1))
                        }
                        ElementState::Released if self.right_pressed => {
                            self.right_pressed = false;
                            Some(TankCommand::Rotate(-1))
                        }
                        _ => None,
                    }
                } else if key == self.fire {
                    match state {
                        ElementState::Pressed if !self.fire_pressed => {
                            self.fire_pressed = true;
                            Some(TankCommand::Fire(true))
                        }
                        ElementState::Released if self.fire_pressed => {
                            self.fire_pressed = false;
                            Some(TankCommand::Fire(false))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

type ReplicaInput = SerdeInputsReplicate<tanks::TankCommand>;
type ReplicaSet = (ReplicaPrefabSerde<TileMapComponent>, Global2);

fn main() {
    game2(|mut game| async move {
        // Create client system to communicate with game server.
        let mut client = ClientSystem::new::<ReplicaInput, ReplicaSet>();

        // Connect to local server. It must be running.
        client
            .connect((Ipv4Addr::LOCALHOST, 12345), &game.scope)
            .await?;

        tracing::info!("Connected");

        // Add player to game session.
        let id = client.add_player((), &game.scope).await?;

        // Add system that will assume control of entities belonging to the added player.
        game.scheduler.add_system(move |cx: SystemContext<'_>| {
            for (e, pid) in cx.world.query_mut::<&PlayerId>().without::<Controlled>() {
                if id == *pid {
                    let controller =
                        EntityController::assume_control(TankComander::main(), 10, e, cx.world)
                            .expect("Entity exists and is not controlled");

                    cx.control.add_global_controller(controller);

                    break;
                }
            }
        });

        tracing::info!("Player added");

        // Set client session to be executed in game loop.
        game.client = Some(client);

        // Setup camera.
        let camera = game.viewport.camera();

        game.world
            .get_mut::<Camera2>(camera)
            .unwrap()
            .set_scaley(0.2);

        // Game configured. Run it.
        Ok(game)
    })
}
