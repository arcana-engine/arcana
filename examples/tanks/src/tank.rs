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
        lifespan::LifeSpan,
        AsyncTaskContext, CommandQueue, ContactQueue2, Global2, InputCommander, PhysicsData2,
        System, SystemContext, TaskContext, TimeSpan,
    },
    eyre::WrapErr as _,
    ordered_float::OrderedFloat,
    rapier2d::{
        dynamics::{RigidBodyBuilder, RigidBodyHandle},
        geometry::{Collider, ColliderBuilder},
        pipeline::ActiveEvents,
    },
    uuid::Uuid,
};

pub struct Bullet;

struct BulletCollider(Collider);

impl BulletCollider {
    fn new() -> Self {
        BulletCollider(
            ColliderBuilder::ball(0.1)
                .active_events(ActiveEvents::CONTACT_EVENTS)
                .build(),
        )
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
    SpriteGraphAnimation::new(
        0,
        sheet,
        vec![
            (TankAnimTransitionRule::AnimationComplete, vec![0], 0),
            (TankAnimTransitionRule::AnimationComplete, vec![1], 1),
            (TankAnimTransitionRule::Moving, vec![0], 1),
            (TankAnimTransitionRule::Broken, vec![0, 1], 2),
            (TankAnimTransitionRule::Idle, vec![1], 0),
        ],
    )
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

        let body = physics.bodies.insert(
            RigidBodyBuilder::new_dynamic()
                .linear_damping(0.3)
                .angular_damping(0.3)
                .build(),
        );

        physics.colliders.insert_with_parent(
            ColliderBuilder::cuboid(hs.x * 0.625, hs.y * 0.6875)
                .active_events(ActiveEvents::CONTACT_EVENTS)
                .build(),
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
            let sprite_sheet = sprite_sheet
                .get(cx.graphics)
                .wrap_err_with(|| "Failed to load tank spritesheet")?;
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

#[derive(Debug)]
pub struct TankState {
    drive: i8,
    rotate: i8,
    fire: bool,
    alive: bool,
}

impl TankState {
    pub fn set_commands(&mut self, commands: impl Iterator<Item = TankCommand>) {
        for cmd in commands {
            match cmd {
                TankCommand::Drive(i) => self.drive += i,
                TankCommand::Rotate(i) => self.rotate += i,
                TankCommand::Fire(fire) => self.fire = fire,
            }
        }
    }
}

#[derive(Debug)]
pub enum TankCommand {
    Drive(i8),
    Rotate(i8),
    Fire(bool),
}

impl InputCommander for TankComander {
    type Command = TankCommand;

    fn translate(&mut self, event: DeviceEvent) -> Option<TankCommand> {
        match event {
            DeviceEvent::Key(KeyboardInput {
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

pub struct TankSystem;

impl System for TankSystem {
    fn name(&self) -> &str {
        "TankSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let physics = cx.res.with(PhysicsData2::new);

        let mut bullets = BVec::new_in(cx.bump);

        for (_entity, (body, global, tank, commands, contacts)) in cx
            .world
            .query::<(
                &RigidBodyHandle,
                &Global2,
                &mut TankState,
                &mut CommandQueue<TankCommand>,
                &mut ContactQueue2,
            )>()
            .with::<Tank>()
            .iter()
        {
            for collider in contacts.drain_contacts_started() {
                let bits = physics.colliders.get(collider).unwrap().user_data as u64;
                let bullet = cx.world.get::<Bullet>(Entity::from_bits(bits)).is_ok();

                if bullet {
                    tank.alive = false;
                }
            }

            if tank.alive {
                tank.set_commands(commands.drain());

                if let Some(body) = physics.bodies.get_mut(*body) {
                    let vel = na::Vector2::new(0.0, -tank.drive as f32);
                    let vel = global.iso.rotation.transform_vector(&vel);

                    body.set_linvel(vel, false);
                    body.set_angvel(tank.rotate as f32 * 3.0, true);
                }

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
                        albedo_factor: [OrderedFloat(1.0), OrderedFloat(0.8), OrderedFloat(0.2)],
                        ..Default::default()
                    },
                    ContactQueue2::new(),
                    LifeSpan::new(TimeSpan::SECOND),
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
        let mut despawn = BVec::new_in(cx.bump);

        for (e, queue) in cx.world.query_mut::<&mut ContactQueue2>().with::<Bullet>() {
            if queue.drain_contacts_started().count() > 0 {
                despawn.push(e);
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
                    LifeSpan::new(TimeSpan::SECOND * 5),
                ));
            }
            let _ = cx.world.despawn(e);
        }

        Ok(())
    }
}

pub type TankAnimationSystem = SpriteGraphAnimationSystem<TankState, TankAnimTransitionRule>;
