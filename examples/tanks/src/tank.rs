use {
    arcana::{
        assets::ImageAsset,
        bumpalo::collections::Vec as BVec,
        event::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode},
        graphics::{Graphics, ImageView, Material, Rect, Sprite, Texture},
        hecs::{Entity, World},
        ContactQueue2, ControlResult, Global2, InputController, PhysicsData2, Prefab, Res, System,
        SystemContext,
    },
    futures::future::BoxFuture,
    goods::{Asset, AssetHandle, AssetResult, Loader},
    ordered_float::OrderedFloat,
    rapier2d::{
        dynamics::{RigidBodyBuilder, RigidBodyHandle},
        geometry::{Collider, ColliderBuilder},
    },
    std::{future::ready, time::Duration},
    uuid::Uuid,
};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Frame {
    pub rect: Rect,
    pub duration_us: u64,
}

#[derive(Clone, Debug, Asset)]
pub struct SpriteSheet {
    pub frames: Vec<Frame>,
    pub animations: Vec<Animation>,

    #[container]
    pub image: Texture,
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub name: Box<str>,
    pub from: usize,
    pub to: usize,
}

pub struct Bullet;

struct BulletCollider(Collider);

impl BulletCollider {
    fn new() -> Self {
        BulletCollider(ColliderBuilder::ball(0.1).build())
    }
}

pub struct Tank {
    size: na::Vector2<f32>,
    color: [f32; 3],
}

impl Tank {
    pub fn spawn(size: na::Vector2<f32>, color: [f32; 3]) -> Self {
        Tank { size, color }
    }
}

impl Prefab for Tank {
    type Loaded = AssetResult<SpriteSheet>;
    type Fut = AssetHandle<SpriteSheet>;

    fn load(&self, loader: &Loader) -> Self::Fut {
        loader.load(&self.sprite_sheet)
    }

    fn spawn(
        mut sprite_sheet: AssetResult<SpriteSheet>,
        res: &mut Res,
        world: &mut World,
        graphics: &mut Graphics,
        entity: Entity,
    ) -> eyre::Result<()> {
        let tank = world.get_mut::<Self>(entity)?;
        let size = tank.size;
        let color = tank.color;
        drop(tank);

        let sprite_sheet = sprite_sheet.get_existing(graphics)?;
        let sampler = graphics.create_sampler(Default::default())?;

        let physics = res.with(PhysicsData2::new);

        let hs = size * 0.5;

        let body = physics
            .bodies
            .insert(RigidBodyBuilder::new_dynamic().build());

        physics.colliders.insert(
            ColliderBuilder::cuboid(hs.x, hs.y).build(),
            body,
            &mut physics.bodies,
        );

        world.insert(
            entity,
            (
                Global2::identity(),
                body,
                Sprite {
                    pos: Rect {
                        left: -hs.x,
                        right: hs.x,
                        top: -hs.y,
                        bottom: hs.y,
                    },
                    uv: Rect {
                        left: 0.0,
                        right: 1.0,
                        top: 0.0,
                        bottom: 1.0,
                    },
                    layer: 1,
                },
                Material {
                    albedo_coverage: Some(Texture {
                        image: sprite_sheet.image.clone(),
                        sampler,
                    }),
                    albedo_factor: [
                        OrderedFloat(color[0]),
                        OrderedFloat(color[1]),
                        OrderedFloat(color[2]),
                    ],
                    ..Default::default()
                },
                SpriteAnimState::new(sprite_sheet),
                ContactQueue2::new(),
            ),
        )?;

        Ok(())
    }
}

#[derive(Clone, Copy)]
struct TankState {
    speed: f32,
    moment: f32,
    fire: bool,
}

pub struct ControlledTank {
    state: TankState,
    newstate: TankState,
}

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

impl InputController for TankController {
    type Controlled = ControlledTank;

    fn controlled(&self) -> ControlledTank {
        ControlledTank {
            state: TankState {
                speed: 0.0,
                moment: 0.0,
                fire: false,
            },
            newstate: TankState {
                speed: 0.0,
                moment: 0.0,
                fire: false,
            },
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
                    tank.newstate.fire = state == ElementState::Pressed;
                } else {
                    return ControlResult::Ignored;
                }

                tank.newstate.speed =
                    3.0 * (self.forward_pressed as u8 as f32 - self.backward_pressed as u8 as f32);

                tank.newstate.moment =
                    3.0 * (self.right_pressed as u8 as f32 - self.left_pressed as u8 as f32);

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

        let mut despawn = BVec::new_in(cx.bump);

        'e: for (entity, (body, global, tank, state, queue)) in cx
            .world
            .query::<(
                &RigidBodyHandle,
                &Global2,
                &mut ControlledTank,
                Option<&mut SpriteAnimState>,
                &mut ContactQueue2,
            )>()
            .with::<Tank>()
            .iter()
        {
            for collider in queue.drain_contacts_started() {
                let bits = physics.colliders.get(collider).unwrap().user_data as u64;
                let bullet = cx.world.get::<Bullet>(Entity::from_bits(bits)).is_ok();

                if bullet {
                    despawn.push(entity);
                    physics
                        .bodies
                        .remove(*body, &mut physics.colliders, &mut physics.joints);
                    continue 'e;
                }
            }

            if let Some(state) = state {
                if tank.newstate.speed > 0.1 && tank.state.speed <= 0.1 {
                    state.set_anim(Anim::Loop { animation: 1 });
                }

                if tank.newstate.speed <= 0.1 && tank.state.speed > 0.1 {
                    state.set_anim(Anim::Loop { animation: 0 });
                }
            }

            if let Some(body) = physics.bodies.get_mut(*body) {
                let vel = na::Vector2::new(0.0, -tank.newstate.speed);
                let vel = global.iso.rotation.transform_vector(&vel);

                body.set_linvel(vel, true);
                body.set_angvel(tank.newstate.moment, true);
            }

            if tank.newstate.fire {
                let pos = global.iso.transform_point(&na::Point2::new(0.0, -0.6));
                let dir = global.iso.transform_vector(&na::Vector2::new(0.0, -10.0));
                bullets.push((pos, dir));
                tank.newstate.fire = false;
            }

            tank.state = tank.newstate;
        }

        for entity in despawn {
            if let Ok(iso) = cx.world.get::<Global2>(entity).map(|g| g.iso) {
                cx.world.spawn((
                    Global2::new(iso),
                    Sprite {
                        pos: Rect {
                            left: -0.5,
                            right: 0.5,
                            top: -0.5,
                            bottom: 0.5,
                        },
                        uv: Rect {
                            left: 0.0,
                            right: 1.0,
                            top: 0.0,
                            bottom: 1.0,
                        },
                        layer: 0,
                    },
                    Material {
                        albedo_factor: [OrderedFloat(0.7), OrderedFloat(0.1), OrderedFloat(0.1)],
                        ..Default::default()
                    },
                ));
            }
            let _ = cx.world.despawn(entity);
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
                        pos: Rect {
                            left: -0.05,
                            right: 0.05,
                            top: -0.05,
                            bottom: 0.05,
                        },
                        uv: Rect {
                            left: 0.0,
                            right: 1.0,
                            top: 0.0,
                            bottom: 1.0,
                        },
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

pub struct SpriteAnimationSystem;

impl System for SpriteAnimationSystem {
    fn name(&self) -> &str {
        "SpriteAnimationSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        for (_, (state, sprite)) in cx.world.query_mut::<(&mut SpriteAnimState, &mut Sprite)>() {
            state.advance(cx.clock.delta);
            sprite.uv = state.get_frame().rect;
        }

        Ok(())
    }
}

#[derive(Debug)]
struct SpriteAnimState {
    current_animation: usize,
    current_frame: usize,
    current_frame_time_us: u64,
    anim: Anim,
    frames: Vec<Frame>,
    animations: Vec<Animation>,
}

impl SpriteAnimState {
    fn new(sheet: &SpriteSheet) -> Self {
        SpriteAnimState {
            current_animation: 0,
            current_frame: 0,
            current_frame_time_us: 0,
            anim: Anim::Loop { animation: 0 },
            frames: sheet.frames.clone(),
            animations: sheet.animations.clone(),
        }
    }

    fn set_anim(&mut self, anim: Anim) {
        match anim {
            Anim::Loop { animation } => {
                self.anim = anim;
                self.current_animation = animation;
                self.current_frame = 0;
                self.current_frame_time_us = 0;
            }
            Anim::RunAndLoop { animation, .. } => {
                self.anim = anim;
                self.current_animation = animation;
                self.current_frame = 0;
                self.current_frame_time_us = 0;
            }
        }
    }

    fn get_frame(&self) -> &Frame {
        let anim = &self.animations[self.current_animation];
        &self.frames[anim.from..=anim.to][self.current_frame]
    }

    fn advance(&mut self, delta: Duration) {
        let mut delta = delta.as_micros() as u64;

        loop {
            let anim = &self.animations[self.current_animation];
            let frames = &self.frames[anim.from..=anim.to];

            if self.current_frame_time_us + delta < frames[self.current_frame].duration_us {
                self.current_frame_time_us += delta;
                return;
            }

            delta -= frames[self.current_frame].duration_us - self.current_frame_time_us;

            self.current_frame += 1;
            self.current_frame_time_us = 0;
            if frames.len() == self.current_frame {
                self.current_frame = 0;

                match self.anim {
                    Anim::Loop { .. } => {}
                    Anim::RunAndLoop { and_loop, .. } => {
                        self.anim = Anim::Loop {
                            animation: and_loop,
                        };
                        self.current_animation = and_loop;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Anim {
    /// Cycle through animations
    Loop {
        animation: usize,
    },
    RunAndLoop {
        animation: usize,
        and_loop: usize,
    },
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
                        pos: Rect {
                            left: -0.2,
                            right: 0.2,
                            top: -0.2,
                            bottom: 0.2,
                        },
                        uv: Rect {
                            left: 0.0,
                            right: 1.0,
                            top: 0.0,
                            bottom: 1.0,
                        },
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
