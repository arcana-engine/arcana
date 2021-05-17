use arcana::TaskContext;

use {
    arcana::{
        anim::{
            graph::{AnimTransitionRule, CurrentAnimInfo},
            sprite::{SpriteGraphAnimation, SpriteGraphAnimationSystem},
        },
        assets::SpriteSheet,
        bumpalo::collections::Vec as BVec,
        event::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode},
        graphics::{Material, Rect, Sprite},
        hecs::Entity,
        AsyncTaskContext, ContactQueue2, ControlResult, Global2, InputController, PhysicsData2,
        System, SystemContext,
    },
    ordered_float::OrderedFloat,
    rapier2d::{
        dynamics::{RigidBodyBuilder, RigidBodyHandle},
        geometry::{Collider, ColliderBuilder},
    },
    uuid::Uuid,
};

pub struct Bullet;

struct BulletCollider(Collider);

impl BulletCollider {
    fn new() -> Self {
        BulletCollider(ColliderBuilder::ball(0.1).build())
    }
}

#[derive(Debug)]
pub enum TankAnimTransitionRule {
    Moving,
    Idle,
    Broken,
    AnimationComplete,
}

impl AnimTransitionRule<TankState> for TankAnimTransitionRule {
    fn matches(&self, state: &TankState, info: &CurrentAnimInfo) -> bool {
        match self {
            Self::Moving => (state.drive != 0 || state.rotate != 0) && state.alive,
            Self::Idle => state.drive == 0 && state.rotate == 0 && state.alive,
            Self::Broken => !state.alive,
            Self::AnimationComplete => info.is_complete(),
        }
    }
}

fn tank_graph_animation(sheet: &SpriteSheet) -> SpriteGraphAnimation<TankAnimTransitionRule> {
    dbg!(SpriteGraphAnimation::new(
        0,
        sheet,
        vec![
            (TankAnimTransitionRule::AnimationComplete, vec![0], 0),
            (TankAnimTransitionRule::AnimationComplete, vec![1], 1),
            (TankAnimTransitionRule::Moving, vec![0], 1),
            (TankAnimTransitionRule::Broken, vec![0, 1], 2),
            (TankAnimTransitionRule::Idle, vec![1], 0),
        ],
    ))
}

pub struct Tank {
    size: na::Vector2<f32>,
    color: [f32; 3],
    sprite_sheet: Uuid,
}

impl Tank {
    pub fn new(size: na::Vector2<f32>, color: [f32; 3], sprite_sheet: Uuid) -> Self {
        Tank {
            size,
            color,
            sprite_sheet,
        }
    }

    /// Spawn this tank.
    pub fn spawn(self, cx: TaskContext<'_>) -> Entity {
        let sprite_sheet = cx.loader.load::<SpriteSheet>(&self.sprite_sheet);

        let physics = cx.res.with(PhysicsData2::new);
        let hs = self.size * 0.5;

        let body = physics
            .bodies
            .insert(RigidBodyBuilder::new_dynamic().build());

        physics.colliders.insert(
            ColliderBuilder::cuboid(hs.x, hs.y).build(),
            body,
            &mut physics.bodies,
        );

        let color = self.color;

        let entity = cx.world.spawn((
            Global2::identity(),
            body,
            Sprite {
                world: Rect {
                    left: -hs.x,
                    right: hs.x,
                    top: -hs.y,
                    bottom: hs.y,
                },
                src: Rect::ONE_QUAD,
                tex: Rect::ONE_QUAD,
                layer: 1,
            },
            ContactQueue2::new(),
            TankState {
                drive: 0,
                rotate: 0,
                alive: true,
                fire: false,
            },
            self,
        ));

        cx.spawner.spawn(async move {
            let mut cx = AsyncTaskContext::new();
            let mut sprite_sheet = sprite_sheet.await;
            let cx = cx.get();
            let sprite_sheet = sprite_sheet.get(cx.graphics)?;
            cx.world.insert(
                entity,
                (
                    tank_graph_animation(&sprite_sheet),
                    Material {
                        albedo_coverage: Some(sprite_sheet.texture.clone()),
                        albedo_factor: [
                            OrderedFloat(color[0]),
                            OrderedFloat(color[1]),
                            OrderedFloat(color[2]),
                        ],
                        ..Default::default()
                    },
                ),
            )?;

            tracing::info!("Tank is fully loaded");

            Ok(())
        });

        entity
    }
}

#[derive(Debug)]
pub struct TankController {
    forward: VirtualKeyCode,
    backward: VirtualKeyCode,
    left: VirtualKeyCode,
    right: VirtualKeyCode,
    fire: VirtualKeyCode,

    forward_pressed: bool,
    backward_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
}

impl TankController {
    pub fn main() -> Self {
        TankController {
            forward: VirtualKeyCode::W,
            backward: VirtualKeyCode::S,
            left: VirtualKeyCode::A,
            right: VirtualKeyCode::D,
            fire: VirtualKeyCode::Space,

            forward_pressed: false,
            backward_pressed: false,
            left_pressed: false,
            right_pressed: false,
        }
    }

    pub fn alt() -> Self {
        TankController {
            forward: VirtualKeyCode::Up,
            backward: VirtualKeyCode::Down,
            left: VirtualKeyCode::Left,
            right: VirtualKeyCode::Right,
            fire: VirtualKeyCode::Insert,

            forward_pressed: false,
            backward_pressed: false,
            left_pressed: false,
            right_pressed: false,
        }
    }
}

#[derive(Debug)]
pub struct TankState {
    drive: i8,
    rotate: i8,
    fire: bool,
    alive: bool,
}

#[derive(Debug)]
pub struct ControlledTank {
    drive: i8,
    rotate: i8,
    fire: bool,
}

impl InputController for TankController {
    type Controlled = ControlledTank;

    fn controlled(&self) -> ControlledTank {
        ControlledTank {
            drive: 0,
            rotate: 0,
            fire: false,
        }
    }

    fn control(&mut self, event: DeviceEvent, tank: &mut ControlledTank) -> ControlResult {
        match event {
            DeviceEvent::Key(KeyboardInput {
                state,
                virtual_keycode: Some(key),
                ..
            }) => {
                let pressed = match state {
                    ElementState::Pressed => true,
                    ElementState::Released => false,
                };

                if key == self.forward {
                    self.forward_pressed = pressed;
                } else if key == self.backward {
                    self.backward_pressed = pressed;
                } else if key == self.left {
                    self.left_pressed = pressed;
                } else if key == self.right {
                    self.right_pressed = pressed;
                } else if key == self.fire {
                    tank.fire = state == ElementState::Pressed;
                } else {
                    return ControlResult::Ignored;
                }

                tank.drive = self.forward_pressed as i8 - self.backward_pressed as i8;
                tank.rotate = self.right_pressed as i8 - self.left_pressed as i8;

                ControlResult::Consumed
            }
            _ => ControlResult::Ignored,
        }
    }
}

pub struct TankSystem;

impl System for TankSystem {
    fn name(&self) -> &str {
        "TankSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let physics = cx.res.with(PhysicsData2::new);

        let mut bullets = BVec::new_in(cx.bump);

        for (entity, (body, global, tank, control, contacts)) in cx
            .world
            .query::<(
                &RigidBodyHandle,
                &Global2,
                &mut TankState,
                &mut ControlledTank,
                &mut ContactQueue2,
            )>()
            .with::<Tank>()
            .iter()
        {
            tank.fire = false;

            for collider in contacts.drain_contacts_started() {
                let bits = physics.colliders.get(collider).unwrap().user_data as u64;
                let bullet = cx.world.get::<Bullet>(Entity::from_bits(bits)).is_ok();

                if bullet {
                    tank.alive = false;

                    physics
                        .bodies
                        .remove(*body, &mut physics.colliders, &mut physics.joints);
                }
            }

            if tank.alive {
                tank.drive = control.drive;
                tank.rotate = control.rotate;

                if let Some(body) = physics.bodies.get_mut(*body) {
                    let vel = na::Vector2::new(0.0, -tank.drive as f32);
                    let vel = global.iso.rotation.transform_vector(&vel);

                    body.set_linvel(vel, true);
                    body.set_angvel(tank.rotate as f32 * 3.0, true);
                }

                if control.fire {
                    let pos = global.iso.transform_point(&na::Point2::new(0.0, -0.6));
                    let dir = global.iso.transform_vector(&na::Vector2::new(0.0, -10.0));
                    bullets.push((pos, dir));
                    tank.fire = true;
                    control.fire = false;
                }
            }
        }

        if !bullets.is_empty() {
            let collider = cx.res.with(BulletCollider::new).0.clone();
            let physics = cx.res.with(PhysicsData2::new);

            for (pos, dir) in bullets {
                let body = physics
                    .bodies
                    .insert(RigidBodyBuilder::new_dynamic().build());
                physics
                    .colliders
                    .insert(collider.clone(), body, &mut physics.bodies);

                physics.bodies.get_mut(body).unwrap().set_linvel(dir, true);

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
                        albedo_factor: [OrderedFloat(1.0), OrderedFloat(0.8), OrderedFloat(0.2)],
                        ..Default::default()
                    },
                    ContactQueue2::new(),
                ));
            }
        }

        Ok(())
    }
}

pub struct BulletSystem;

impl System for BulletSystem {
    fn name(&self) -> &str {
        "BulletSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let physics = cx.res.with(PhysicsData2::new);
        let mut despawn = BVec::new_in(cx.bump);

        for (e, (queue, body)) in cx
            .world
            .query_mut::<(&mut ContactQueue2, &RigidBodyHandle)>()
            .with::<Bullet>()
        {
            if queue.drain_contacts_started().count() > 0 {
                despawn.push(e);

                physics
                    .bodies
                    .remove(*body, &mut physics.colliders, &mut physics.joints);
            }
            queue.drain_contacts_stopped();
        }

        for e in despawn {
            if let Ok(iso) = cx.world.get::<Global2>(e).map(|g| g.iso) {
                cx.world.spawn((
                    Global2::new(iso),
                    Sprite {
                        world: Rect {
                            left: -0.2,
                            right: 0.2,
                            top: -0.2,
                            bottom: 0.2,
                        },
                        src: Rect::ONE_QUAD,
                        tex: Rect::ONE_QUAD,
                        layer: 0,
                    },
                    Material {
                        albedo_factor: [OrderedFloat(1.0), OrderedFloat(0.3), OrderedFloat(0.1)],
                        ..Default::default()
                    },
                ));
            }
            let _ = cx.world.despawn(e);
        }

        Ok(())
    }
}

pub type TankAnimationSystem = SpriteGraphAnimationSystem<TankState, TankAnimTransitionRule>;
