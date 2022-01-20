#![feature(allocator_api)]

use arcana::{
    assets::WithId,
    hecs::{Entity, World},
    lifespan::LifeSpan,
    na,
    physics2::{ContactQueue2, PhysicsData2},
    prelude::*,
    rapier2d::prelude::{
        ActiveEvents, Collider, ColliderBuilder, RigidBodyBuilder, RigidBodyHandle,
    },
    scoped_arena::Scope,
    sprite::graph::{AnimTransitionRule, CurrentAnimInfo},
};

cfg_if::cfg_if! {
    if #[cfg(feature = "graphics")] {
        use arcana::{
            graphics::{Material, Texture},
            rect::Rect,
            assets::AssetLoadCache,
            sprite::{
                anim::{SpriteGraphAnimation, SpriteGraphAnimationSystem},
                sprite_sheet::{SpriteSheet, SpriteSheetMeta},
                Sprite,
            },
        };
        use ordered_float::OrderedFloat;
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

#[cfg(feature = "graphics")]
fn tank_sprite_sheet_id(tank: &Tank) -> AssetId {
    WithId::id(&tank.sprite_sheet)
}

#[cfg(not(feature = "graphics"))]
fn tank_sprite_sheet_id(tank: &Tank) -> AssetId {
    tank.sprite_sheet
}

#[derive(Clone, Debug, Asset)]
#[asset(name = "tank")]
pub struct Tank {
    size: na::Vector2<f32>,
    color: [f32; 3],

    #[cfg(feature = "graphics")]
    #[asset(external)]
    sprite_sheet: WithId<SpriteSheet<Texture>>,

    #[cfg(not(feature = "graphics"))]
    sprite_sheet: AssetId,
}

impl Tank {
    pub fn new(
        size: na::Vector2<f32>,
        color: [f32; 3],
        #[cfg(feature = "graphics")] sprite_sheet: WithId<SpriteSheet<Texture>>,
        #[cfg(not(feature = "graphics"))] sprite_sheet: AssetId,
    ) -> Self {
        Tank {
            size,
            color,

            sprite_sheet: sprite_sheet.into(),
        }
    }

    /// Spawn this tank.
    pub fn spawn(self, iso: &na::Isometry2<f32>, world: &mut World, res: &mut Res) -> Entity {
        let physics = res.with(PhysicsData2::new);
        let hs = self.size * 0.5;

        #[cfg(feature = "graphics")]
        let sprite_sheet = &self.sprite_sheet;

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
            ContactQueue2::new(),
            TankState {
                drive: 0,
                rotate: 0,
                alive: true,
                fire: false,
            },
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
            tank_graph_animation(&sprite_sheet),
            #[cfg(feature = "graphics")]
            Material {
                albedo_coverage: Some(sprite_sheet.texture.clone()),
                albedo_factor: [
                    OrderedFloat(color[0]),
                    OrderedFloat(color[1]),
                    OrderedFloat(color[2]),
                    OrderedFloat(1.0),
                ],
                ..Default::default()
            },
            self,
        ));

        tracing::info!("Tank is fully loaded");

        entity
    }
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
                        albedo_factor: [
                            OrderedFloat(1.0),
                            OrderedFloat(0.8),
                            OrderedFloat(0.2),
                            OrderedFloat(1.0),
                        ],
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
                        albedo_factor: [
                            OrderedFloat(1.0),
                            OrderedFloat(0.3),
                            OrderedFloat(0.1),
                            OrderedFloat(1.0),
                        ],
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

#[cfg(feature = "graphics")]
pub type TankAnimationSystem = SpriteGraphAnimationSystem<TankState, TankAnimTransitionRule>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TankReplica {
    pub size: na::Vector2<f32>,
    pub color: [f32; 3],
    pub sprite_sheet: AssetId,
    pub state: TankState,
}

impl TankReplica {
    fn from_tank_state(tank: &Tank, state: &TankState) -> Self {
        TankReplica {
            size: tank.size,
            color: tank.color,
            sprite_sheet: tank_sprite_sheet_id(tank),
            state: *state,
        }
    }

    fn equivalent(&self, tank: &Tank, state: &TankState) -> bool {
        self.size == tank.size
            && self.color == tank.color
            && self.sprite_sheet == tank_sprite_sheet_id(tank)
            && self.state == *state
    }
}

#[cfg(feature = "graphics")]
type TankReplicaCache = AssetLoadCache<SpriteSheet<Texture>>;

pub struct TankReplicaSystem;

impl System for TankReplicaSystem {
    fn name(&self) -> &str {
        "TankReplicaSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        #[cfg(feature = "graphics")]
        let cache = cx.res.with(TankReplicaCache::new);

        let mut spawn = Vec::new_in(&*cx.scope);
        let mut remove_replica = Vec::new_in(&*cx.scope);

        #[cfg(feature = "graphics")]
        type MaterialFetch = Option<&'static mut Material>;

        #[cfg(not(feature = "graphics"))]
        type MaterialFetch = ();

        let query = cx.world.query_mut::<(
            &Global2,
            &TankReplica,
            Option<&mut Tank>,
            Option<&mut TankState>,
            MaterialFetch,
        )>();

        for (entity, (global, replica, tank, state, mat)) in query {
            match tank {
                Some(tank) if tank_sprite_sheet_id(&tank) == replica.sprite_sheet => {
                    *state.unwrap() = replica.state;

                    #[cfg(feature = "graphics")]
                    let () = mat.unwrap().albedo_factor = [
                        OrderedFloat(replica.color[0]),
                        OrderedFloat(replica.color[1]),
                        OrderedFloat(replica.color[2]),
                        OrderedFloat(1.0),
                    ];

                    remove_replica.push(entity);
                    continue;
                }
                _ => {}
            }

            #[cfg(feature = "graphics")]
            match &tank {
                Some(tank) if tank_sprite_sheet_id(&tank) == replica.sprite_sheet => {}
                _ => {
                    cache.load(replica.sprite_sheet, cx.loader);
                }
            }

            #[cfg(feature = "graphics")]
            match tank {
                None => {
                    if let Some(sheet) = cache.get_ready(replica.sprite_sheet) {
                        spawn.push((entity, global.iso, sheet.clone()));
                    }
                }
                Some(tank) => {
                    if tank_sprite_sheet_id(&tank) != replica.sprite_sheet {
                        if let Some(sheet) = cache.get_ready(replica.sprite_sheet) {
                            spawn.push((entity, global.iso, sheet.clone()));
                        }
                    } else {
                        spawn.push((entity, global.iso, (&*tank.sprite_sheet).clone()));
                    }
                }
            }

            #[cfg(not(feature = "graphics"))]
            spawn.push((entity, global.iso, ()));
        }

        #[cfg(feature = "graphics")]
        cache.ensure_task(cx.spawner, |cx| cx.graphics);

        for (entity, iso, sheet) in spawn {
            let replica = cx.world.remove_one::<TankReplica>(entity).unwrap();

            let tank = Tank {
                #[cfg(feature = "graphics")]
                sprite_sheet: WithId::new(sheet, replica.sprite_sheet),
                #[cfg(not(feature = "graphics"))]
                sprite_sheet: replica.sprite_sheet,
                color: replica.color,
                size: replica.size,
            };
            let state = replica.state;

            let physics = cx.res.with(PhysicsData2::new);
            let hs = tank.size * 0.5;

            #[cfg(feature = "graphics")]
            let sprite_sheet = &tank.sprite_sheet;

            let body = physics.bodies.insert(
                RigidBodyBuilder::new_dynamic()
                    .linear_damping(0.3)
                    .angular_damping(0.3)
                    .position(iso)
                    .build(),
            );

            physics.colliders.insert_with_parent(
                ColliderBuilder::cuboid(hs.x * 0.625, hs.y * 0.6875)
                    .active_events(ActiveEvents::CONTACT_EVENTS)
                    .build(),
                body,
                &mut physics.bodies,
            );

            cx.world
                .insert(
                    entity,
                    (
                        body,
                        ContactQueue2::new(),
                        state,
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
                        tank_graph_animation(&sprite_sheet),
                        #[cfg(feature = "graphics")]
                        Material {
                            albedo_coverage: Some(sprite_sheet.texture.clone()),
                            albedo_factor: [
                                OrderedFloat(tank.color[0]),
                                OrderedFloat(tank.color[1]),
                                OrderedFloat(tank.color[2]),
                                OrderedFloat(1.0),
                            ],
                            ..Default::default()
                        },
                        tank,
                    ),
                )
                .unwrap();
        }

        for entity in remove_replica {
            let _ = cx.world.remove_one::<TankReplica>(entity);
        }

        #[cfg(feature = "graphics")]
        {
            let cache = cx.res.get_mut::<TankReplicaCache>().unwrap();
            cache.clear_ready();
        }

        Ok(())
    }
}

#[cfg(any(feature = "client", feature = "server"))]
pub enum TankDescriptor {}

#[cfg(feature = "client")]
impl evoke::client::Descriptor for TankDescriptor {
    type Query = &'static mut TankReplica;
    type Pack = TankReplica;

    fn insert(pack: TankReplica, entity: Entity, world: &mut World) {
        let _ = world.insert_one(entity, pack);
    }

    fn modify(pack: TankReplica, item: &mut TankReplica) {
        *item = pack;
    }

    fn remove(entity: Entity, world: &mut World) {
        let _ = world.remove_one::<TankReplica>(entity);

        #[cfg(feature = "graphics")]
        type Bundle = (
            Tank,
            TankState,
            RigidBodyHandle,
            ContactQueue2,
            Material,
            Sprite,
            SpriteGraphAnimation<TankAnimTransitionRule>,
        );

        #[cfg(not(feature = "graphics"))]
        type Bundle = (Tank, TankState, RigidBodyHandle, ContactQueue2);

        if let Err(arcana::hecs::ComponentError::MissingComponent(component)) =
            world.remove::<Bundle>(entity)
        {
            tracing::error!("Tank components missing: '{}'", component);

            let _ = world.remove_one::<Tank>(entity);
            let _ = world.remove_one::<TankState>(entity);
            let _ = world.remove_one::<ContactQueue2>(entity);
            let _ = world.remove_one::<RigidBodyHandle>(entity);

            #[cfg(feature = "graphics")]
            let _ = world.remove_one::<Material>(entity);

            #[cfg(feature = "graphics")]
            let _ = world.remove_one::<Sprite>(entity);

            #[cfg(feature = "graphics")]
            let _ = world.remove_one::<SpriteGraphAnimation<TankAnimTransitionRule>>(entity);
        }
    }
}
#[cfg(feature = "server")]
impl evoke::server::DescriptorPack<'_> for TankDescriptor {
    type Pack = TankReplica;
}

#[cfg(feature = "server")]
impl evoke::server::Descriptor for TankDescriptor {
    type Query = (&'static Tank, &'static TankState);
    type History = TankReplica;

    fn history((tank, state): (&Tank, &TankState)) -> TankReplica {
        TankReplica::from_tank_state(tank, state)
    }

    fn replicate<'a>(
        (tank, state): (&Tank, &TankState),
        history: Option<&TankReplica>,
        _scope: &'a Scope<'_>,
    ) -> evoke::server::Replicate<TankReplica> {
        match history {
            Some(history) if history.equivalent(tank, state) => {
                evoke::server::Replicate::Unmodified
            }
            _ => evoke::server::Replicate::Modified(TankReplica::from_tank_state(tank, state)),
        }
    }
}
