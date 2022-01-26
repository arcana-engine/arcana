#![feature(allocator_api)]

use arcana::{
    hecs::Entity,
    lifespan::LifeSpan,
    na,
    physics2::{ContactQueue2, PhysicsData2},
    prelude::*,
    rapier2d::prelude::{
        ActiveEvents, Collider, ColliderBuilder, RigidBodyBuilder, RigidBodyHandle,
    },
    unfold::{UnfoldBundle, UnfoldResult},
};

#[cfg(feature = "client")]
use arcana::hecs::World;

#[cfg(feature = "server")]
use arcana::scoped_arena::Scope;

cfg_if::cfg_if! {
    if #[cfg(feature = "graphics")] {
        use arcana::{
            assets::WithId,
            graphics::{Material, Texture},
            rect::Rect,
            sprite::{
                anim::{SpriteGraphAnimation, SpriteGraphAnimationSystem},
                sprite_sheet::{SpriteSheet, SpriteSheetMeta},
                Sprite,graph::{AnimTransitionRule, CurrentAnimInfo}
            },
        };
    }
}

#[cfg(any(feature = "client", feature = "server"))]
use arcana::evoke;

use arcana::assets::Asset;
use goods::AssetId;

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

#[cfg(feature = "graphics")]
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

#[cfg(feature = "graphics")]
fn tank_graph_animation(sheet: &SpriteSheetMeta) -> SpriteGraphAnimation<TankAnimTransitionRule> {
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

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TankState {
    drive: i8,
    rotate: i8,
    fire: bool,
    alive: bool,
}

impl TankState {
    pub fn new() -> Self {
        TankState {
            drive: 0,
            rotate: 0,
            fire: false,
            alive: true,
        }
    }

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

    fn run(&mut self, cx: SystemContext<'_>) {
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
                    #[cfg(feature = "graphics")]
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
                    #[cfg(feature = "graphics")]
                    Material {
                        albedo_factor: [1.0, 0.8, 0.2, 1.0],
                        ..Default::default()
                    },
                    ContactQueue2::new(),
                    LifeSpan::new(TimeSpan::SECOND),
                ));
            }
        }
    }
}

#[cfg(feature = "server")]
pub struct TankSystem;

#[cfg(feature = "server")]
impl System for TankSystem {
    fn name(&self) -> &str {
        "TankSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
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
                if let Some(entity) = Entity::from_bits(bits) {
                    let bullet = cx.world.get_mut::<Bullet>(entity).is_ok();
                    if bullet {
                        tank.alive = false;
                    }
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
                    #[cfg(feature = "graphics")]
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
                    #[cfg(feature = "graphics")]
                    Material {
                        albedo_factor: [1.0, 0.8, 0.2, 1.0],
                        ..Default::default()
                    },
                    ContactQueue2::new(),
                    LifeSpan::new(TimeSpan::SECOND),
                ));
            }
        }
    }
}

pub struct BulletSystem;

impl System for BulletSystem {
    fn name(&self) -> &str {
        "BulletSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        let mut despawn = Vec::new_in(&*cx.scope);

        for (e, queue) in cx.world.query_mut::<&mut ContactQueue2>().with::<Bullet>() {
            if queue.drain_contacts_started().count() > 0 {
                despawn.push(e);
            }
            queue.drain_contacts_stopped();
        }

        for e in despawn {
            #[cfg(feature = "graphics")]
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
                        albedo_factor: [1.0, 0.3, 0.1, 1.0],
                        ..Default::default()
                    },
                    LifeSpan::new(TimeSpan::SECOND * 5),
                ));
            }
            let _ = cx.world.despawn(e);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Asset)]
#[asset(name = "tank")]
#[derive(Unfold)]
#[unfold(fn unfold_tank)]
pub struct Tank {
    pub size: na::Vector2<f32>,
    pub color: [f32; 3],

    #[cfg_attr(feature = "graphics", unfold(asset: SpriteSheet<Texture>))]
    pub sprite_sheet: AssetId,
}

#[allow(unused_variables)]
fn unfold_tank(
    size: &na::Vector2<f32>,
    color: &[f32; 3],
    #[cfg(feature = "graphics")] sprite_sheet: &WithId<SpriteSheet<Texture>>,
    #[cfg(not(feature = "graphics"))] _sprite_sheet: &AssetId,
    res: &mut Res,
) -> UnfoldResult<impl UnfoldBundle> {
    let hs = size / 2.0;
    let physics = res.with(PhysicsData2::new);

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

    UnfoldResult::with_bundle((
        body,
        ContactQueue2::new(),
        #[cfg(feature = "graphics")]
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
        #[cfg(feature = "graphics")]
        Material {
            albedo_coverage: Some(sprite_sheet.texture.clone()),
            albedo_factor: [color[0], color[1], color[2], 1.0],
            ..Default::default()
        },
        #[cfg(feature = "graphics")]
        tank_graph_animation(&sprite_sheet),
    ))
}

#[cfg(any(feature = "client", feature = "server"))]
pub enum TankDescriptor {}

#[cfg(feature = "client")]
impl evoke::client::Descriptor for TankDescriptor {
    type Query = (&'static mut Tank, &'static mut TankState);
    type Pack = (Tank, TankState);

    fn insert(pack: (Tank, TankState), entity: Entity, world: &mut World) {
        let _ = world.insert(entity, pack);
    }

    fn modify(pack: (Tank, TankState), (tank, state): (&mut Tank, &mut TankState)) {
        *tank = pack.0;
        *state = pack.1;
    }

    fn remove(entity: Entity, world: &mut World) {
        let _ = world.remove_one::<Tank>(entity);
        let _ = world.remove_one::<TankState>(entity);
    }
}
#[cfg(feature = "server")]
impl evoke::server::DescriptorPack<'_> for TankDescriptor {
    type Pack = (Tank, TankState);
}

#[cfg(feature = "server")]
impl evoke::server::Descriptor for TankDescriptor {
    type Query = (&'static Tank, &'static TankState);
    type History = (Tank, TankState);

    fn history((tank, state): (&Tank, &TankState)) -> (Tank, TankState) {
        (*tank, *state)
    }

    fn replicate<'a>(
        (tank, state): (&Tank, &TankState),
        history: Option<&(Tank, TankState)>,
        _scope: &'a Scope<'_>,
    ) -> evoke::server::Replicate<(Tank, TankState)> {
        match history {
            Some((htank, hstate)) if htank == tank && hstate == state => {
                evoke::server::Replicate::Unmodified
            }
            _ => evoke::server::Replicate::Modified((*tank, *state)),
        }
    }
}

#[cfg(feature = "graphics")]
pub type TankAnimationSystem = SpriteGraphAnimationSystem<TankState, TankAnimTransitionRule>;
