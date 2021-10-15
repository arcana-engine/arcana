#![feature(allocator_api)]

use alkahest::Schema;

use arcana::{
    hecs::{Entity, World},
    lifespan::LifeSpan,
    na,
    physics2::{
        dynamics::{RigidBodyBuilder, RigidBodyHandle},
        geometry::{Collider, ColliderBuilder},
        pipeline::ActiveEvents,
        ContactQueue2, PhysicsData2,
    },
    uuid::Uuid,
    with_async_task_context, CommandQueue, Global2, Res, Spawner, System, SystemContext, TimeSpan,
};
use eyre::WrapErr;

#[cfg(feature = "client")]
use arcana::net::client;

#[cfg(feature = "server")]
use arcana::net::server;

use ordered_float::OrderedFloat;

#[cfg(feature = "visible")]
use arcana::{
    anim::{
        graph::{AnimTransitionRule, CurrentAnimInfo},
        sprite::{SpriteGraphAnimation, SpriteGraphAnimationSystem},
    },
    assets::SpriteSheet,
    graphics::{Material, Rect, Sprite},
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

#[cfg(feature = "visible")]
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

#[cfg(feature = "visible")]
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

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn spawn(
        self,
        iso: &na::Isometry2<f32>,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
    ) -> Entity {
        let physics = res.with(PhysicsData2::new);
        let hs = self.size * 0.5;

        #[cfg(feature = "visible")]
        let sprite_sheet = self.sprite_sheet;

        let body = physics.bodies.insert(
            RigidBodyBuilder::new_dynamic()
                .linear_damping(0.3)
                .angular_damping(0.3)
                .position(*iso)
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

        let entity = world.spawn((
            Global2::new(*iso),
            body,
            #[cfg(feature = "visible")]
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

        #[cfg(feature = "visible")]
        spawner.spawn(async move {
            let mut sprite_sheet =
                with_async_task_context(|cx| cx.loader.load::<SpriteSheet>(&sprite_sheet)).await;

            with_async_task_context(|cx| {
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
            })
        });

        entity
    }
}

#[cfg(feature = "client")]
impl client::SelfDescriptor for Tank {
    fn modify(&mut self, new: Self, entity: Entity, spawner: &mut Spawner) {
        // Note modified fields
        #[cfg(feature = "visible")]
        let modify_sprite = self.sprite_sheet != new.sprite_sheet;

        #[cfg(feature = "visible")]
        let modify_color = self.color != new.color;

        let modify_size = self.size != new.size;

        // Update component.
        *self = new;

        #[cfg(feature = "visible")]
        let color = self.color;

        #[cfg(feature = "visible")]
        let sprite_sheet = self.sprite_sheet;

        // Process modifications.
        if modify_size {
            let size = self.size;

            spawner.spawn(async move {
                with_async_task_context(|cx| {
                    let physics = cx.res.with(PhysicsData2::new);
                    let hs = size * 0.5;

                    let body = physics
                        .bodies
                        .insert(RigidBodyBuilder::new_kinematic_position_based().build());

                    physics.colliders.insert_with_parent(
                        ColliderBuilder::cuboid(hs.x * 0.625, hs.y * 0.6875)
                            .active_events(ActiveEvents::CONTACT_EVENTS)
                            .build(),
                        body,
                        &mut physics.bodies,
                    );

                    let _ = cx.world.insert(
                        entity,
                        (
                            body,
                            #[cfg(feature = "visible")]
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
                        ),
                    );
                });

                #[cfg(feature = "visible")]
                {
                    if modify_sprite {
                        let mut sprite_sheet = with_async_task_context(|cx| {
                            cx.loader.load::<SpriteSheet>(&sprite_sheet)
                        })
                        .await;

                        with_async_task_context(|cx| {
                            let sprite_sheet = sprite_sheet
                                .get(cx.graphics)
                                .wrap_err_with(|| "Failed to load tank spritesheet")?;

                            let _ = cx.world.insert(
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
                            );
                            Ok::<_, eyre::Report>(())
                        })?;
                    } else if modify_color {
                        with_async_task_context(|cx| {
                            if let Ok(material) = cx.world.query_one_mut::<&mut Material>(entity) {
                                material.albedo_factor = [
                                    OrderedFloat(color[0]),
                                    OrderedFloat(color[1]),
                                    OrderedFloat(color[2]),
                                ];
                            }
                        })
                    }
                }
                Ok(())
            })
        } else {
            #[cfg(feature = "visible")]
            {
                if modify_sprite {
                    spawner.spawn(async move {
                        let mut sprite_sheet = with_async_task_context(|cx| {
                            cx.loader.load::<SpriteSheet>(&sprite_sheet)
                        })
                        .await;

                        with_async_task_context(|cx| {
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

                            Ok(())
                        })
                    });
                } else if modify_color {
                    spawner.spawn(async move {
                        with_async_task_context(|cx| {
                            if let Ok(material) = cx.world.query_one_mut::<&mut Material>(entity) {
                                material.albedo_factor = [
                                    OrderedFloat(color[0]),
                                    OrderedFloat(color[1]),
                                    OrderedFloat(color[2]),
                                ];
                            }
                        });
                        Ok(())
                    })
                }
            }
        }
    }

    fn insert(self, entity: Entity, world: &mut World, res: &mut Res, spawner: &mut Spawner) {
        let physics = res.with(PhysicsData2::new);
        let hs = self.size * 0.5;

        let sprite_sheet = self.sprite_sheet;

        let body = physics
            .bodies
            .insert(RigidBodyBuilder::new_kinematic_position_based().build());

        physics.colliders.insert_with_parent(
            ColliderBuilder::cuboid(hs.x * 0.625, hs.y * 0.6875)
                .active_events(ActiveEvents::CONTACT_EVENTS)
                .build(),
            body,
            &mut physics.bodies,
        );

        let color = self.color;

        world
            .insert(
                entity,
                (
                    body,
                    #[cfg(feature = "visible")]
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
                    self,
                ),
            )
            .unwrap();

        #[cfg(feature = "visible")]
        spawner.spawn(async move {
            let mut sprite_sheet =
                with_async_task_context(|cx| cx.loader.load::<SpriteSheet>(&sprite_sheet)).await;

            with_async_task_context(|cx| {
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

                Ok(())
            })
        });
    }

    fn on_remove(&mut self, _entity: Entity, _spawner: &mut Spawner) {}
}

#[cfg(feature = "server")]
impl server::TrivialDescriptor for Tank {}

#[derive(Clone, Copy, Debug, Schema, PartialEq, serde::Serialize, serde::Deserialize)]
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
                TankCommand::Drive(i) => self.drive = self.drive.saturating_add(i),
                TankCommand::Rotate(i) => self.rotate = self.rotate.saturating_add(i),
                TankCommand::Fire(fire) => self.fire = fire,
            }
        }
    }
}

#[cfg(feature = "client")]
impl client::TrivialDescriptor for TankState {}

#[cfg(feature = "server")]
impl server::TrivialDescriptor for TankState {}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum TankCommand {
    Drive(i8),
    Rotate(i8),
    Fire(bool),
}

#[cfg(feature = "client")]
pub struct TankClientSystem;

#[cfg(feature = "client")]
impl System for TankClientSystem {
    fn name(&self) -> &str {
        "TankClientSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let mut bullets = Vec::new_in(&*cx.scope);

        for (_entity, (global, tank)) in cx
            .world
            .query::<(&Global2, &mut TankState)>()
            .with::<Tank>()
            .iter()
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
                    #[cfg(feature = "visible")]
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
                    #[cfg(feature = "visible")]
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

#[cfg(feature = "server")]
pub struct TankSystem;

#[cfg(feature = "server")]
impl System for TankSystem {
    fn name(&self) -> &str {
        "TankSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let physics = cx.res.with(PhysicsData2::new);

        let mut bullets = Vec::new_in(&*cx.scope);

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
                let bullet = cx.world.get_mut::<Bullet>(Entity::from_bits(bits)).is_ok();

                if bullet {
                    tank.alive = false;
                }
            }

            if tank.alive {
                tank.fire = false;
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
                    #[cfg(feature = "visible")]
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
                    #[cfg(feature = "visible")]
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
        let mut despawn = Vec::new_in(&*cx.scope);

        for (e, queue) in cx.world.query_mut::<&mut ContactQueue2>().with::<Bullet>() {
            if queue.drain_contacts_started().count() > 0 {
                despawn.push(e);
            }
            queue.drain_contacts_stopped();
        }

        for e in despawn {
            #[cfg(feature = "visible")]
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

#[cfg(feature = "visible")]
pub type TankAnimationSystem = SpriteGraphAnimationSystem<TankState, TankAnimTransitionRule>;
