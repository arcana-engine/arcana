#![feature(allocator_api)]

use std::{
    net::Ipv4Addr,
    sync::atomic::{AtomicU32, Ordering},
};

use arcana::{
    assets::{image::QoiImage, AssetId},
    edict::{entity::EntityId, world::World},
    evoke, na,
    palette::*,
    physics2::{ContactQueue2, Physics2, PhysicsData2},
    prelude::*,
    rapier2d::prelude::{RigidBodyBuilder, RigidBodyHandle},
    sprite::SpriteSheet,
    tiles::{TileMap, TileSet},
};
use eyre::WrapErr;
use tokio::net::TcpListener;

use tanks::*;

fn random_color() -> [f32; 3] {
    const FI: f32 = 1.618033988;
    static COLOR_WHEEL: AtomicU32 = AtomicU32::new(0);
    let color_wheel = COLOR_WHEEL.fetch_add(1, Ordering::Relaxed);

    let hue = ((color_wheel as f32) * FI).fract();
    let lch = Lch::new(100.0, 128.0, hue * 360.0);
    let rgb = Srgb::from_color(lch);

    tracing::error!("{:#?}", rgb);

    [rgb.red, rgb.green, rgb.blue]
}

struct RemoteTankPlayer {
    entity: EntityId,
}

impl evoke::server::RemotePlayer for RemoteTankPlayer {
    type Input = Vec<tanks::TankCommand>;
    type Info = ();

    fn accept((): (), pid: evoke::PlayerId, world: &mut World) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        let (_, &sprite_sheet) = world
            .query::<&TankSpriteSheetId>()
            .into_iter()
            .next()
            .unwrap();

        let pos = random_spawn_location(world);
        let entity = world.spawn((
            evoke::server::ServerOwned,
            pid,
            pos,
            Tank {
                size: na::Vector2::new(1.0, 1.0),
                color: random_color(),
                sprite_sheet: sprite_sheet.0,
            },
            TankState::new(),
            TankStateInternal::new(),
            CommandQueue::<TankCommand>::new(),
        ));

        tracing::info!("Player's tank spawned");

        Ok(RemoteTankPlayer { entity })
    }

    #[inline(always)]
    fn disconnected(self, world: &mut World)
    where
        Self: Sized,
    {
        tracing::error!("Set 5s TTL for tank of disconnected player");

        const KEEP_DISCONNECTED_FOR: TimeSpan = TimeSpan::from_seconds(5);

        match world.query_one_mut::<&mut LifeSpan>(&self.entity) {
            Ok(lifespan) => lifespan.truncate(KEEP_DISCONNECTED_FOR),
            _ => {
                let _ = world.try_insert(&self.entity, LifeSpan::new(KEEP_DISCONNECTED_FOR));
            }
        }
    }

    fn apply_input(&mut self, entity: EntityId, world: &mut World, pack: Vec<tanks::TankCommand>) {
        if self.entity == entity {
            if let Ok(queue) = world.query_one_mut::<&mut CommandQueue<_>>(&entity) {
                queue.enque(pack);
            }
        } else {
            tracing::error!("Player attempts to control not-owned entity");
        }
    }
}

pub struct TankStateInternal {
    reload: TimeSpan,
    last_fire: TimeStamp,
    pending_fire: bool,
}

impl TankStateInternal {
    pub fn new() -> Self {
        TankStateInternal {
            reload: TimeSpan::SECOND,
            last_fire: TimeStamp::ORIGIN,
            pending_fire: false,
        }
    }
}

struct Respawner {
    tank: EntityId,
    timeout: TimeStamp,
}

pub struct TankSystem;

impl System for TankSystem {
    fn name(&self) -> &str {
        "TankSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        let physics = cx.res.with(PhysicsData2::new);

        let mut bullets = Vec::new_in(&*cx.scope);
        let mut respawners = Vec::new_in(&*cx.scope);

        let (meta, query) = cx.world.meta_query_mut::<(
            &RigidBodyHandle,
            &Global2,
            &mut TankState,
            &mut TankStateInternal,
            &mut CommandQueue<TankCommand>,
            &mut ContactQueue2,
        )>();

        for (entity, (body, global, tank, internal, commands, contacts)) in query.with::<Tank>() {
            for collider in contacts.drain_contacts_started() {
                let bits = physics.colliders.get(collider).unwrap().user_data as u64;
                if let Some(collider_entity) = EntityId::from_bits(bits) {
                    if meta
                        .has_component::<Bullet>(&collider_entity)
                        .unwrap_or(false)
                    {
                        tank.alive = false;
                        respawners.push(Respawner {
                            tank: entity,
                            timeout: cx.clock.now + timespan!(5s),
                        });
                    }
                }
            }

            if tank.alive {
                tank.fire = false;

                for cmd in commands.drain() {
                    match cmd {
                        TankCommand::Drive(i) => tank.drive = tank.drive.saturating_add(i),
                        TankCommand::Rotate(i) => tank.rotate = tank.rotate.saturating_add(i),
                        TankCommand::Fire => {
                            if internal.last_fire + internal.reload
                                <= cx.clock.now + internal.reload / 4
                            {
                                internal.pending_fire = true;
                            }
                        }
                    }
                }

                if internal.pending_fire && internal.last_fire + internal.reload <= cx.clock.now {
                    tank.fire = true;
                    internal.pending_fire = false;
                    internal.last_fire = cx.clock.now;
                }

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
                    ContactQueue2::new(),
                    LifeSpan::new(TimeSpan::SECOND * 3),
                ));
            }
        }

        let mut remove_respawners = Vec::new_in(&*cx.scope);
        let mut respawn_tanks = Vec::new_in(&*cx.scope);

        for (e, respawner) in cx.world.query_mut::<&Respawner>() {
            if respawner.timeout < cx.clock.now {
                remove_respawners.push(e);
                respawn_tanks.push(respawner.tank);
            }
        }

        for e in remove_respawners {
            let _ = cx.world.despawn(&e);
        }

        for e in respawn_tanks {
            let spawn_at = random_spawn_location(cx.world);

            if let Ok((tank, internal, global)) =
                cx.world
                    .query_one_mut::<(&mut TankState, &mut TankStateInternal, &mut Global2)>(&e)
            {
                *tank = TankState::new();
                *internal = TankStateInternal::new();
                *global = spawn_at;
            }
        }

        for spawner in respawners {
            cx.world.spawn((spawner,));
        }
    }
}

#[derive(Clone, Copy)]
struct TankSpriteSheetId(AssetId);

fn main() {
    headless(|mut game| async move {
        let tank_asset_id = game
            .assets
            .lookup::<SpriteSheet<QoiImage>>("tank.json")
            .await?;

        game.world.spawn((TankSpriteSheetId(tank_asset_id),));

        let maps = [
            game.assets
                .load::<TileMap, _>("tanks-map1.json")
                .await
                .get()
                .wrap_err("Failed to load tile map")?
                .clone(),
            game.assets
                .load::<TileMap, _>("tanks-map2.json")
                .await
                .get()
                .wrap_err("Failed to load tile map")?
                .clone(),
            game.assets
                .load::<TileMap, _>("tanks-map3.json")
                .await
                .get()
                .wrap_err("Failed to load tile map")?
                .clone(),
        ];

        for i in -1..=1 {
            for j in -1..=1 {
                let index = rand::random::<usize>() % maps.len();
                let map = &maps[index];

                let offset = na::Vector2::new(i as f32, j as f32).component_mul(&map.size());

                game.world.spawn((
                    Global2::new(na::Isometry2::new(offset.into(), 0.0)),
                    map.clone(),
                    evoke::server::ServerOwned,
                ));
            }
        }

        TileMap::schedule_unfold_system(&mut game.scheduler);
        Tank::schedule_unfold_system(&mut game.scheduler);

        game.scheduler.add_ticking_system(Physics2::new());
        game.scheduler.add_ticking_system(TankSystem);
        game.scheduler.add_ticking_system(tanks::BulletSystem);

        // Bind listener for incoming connections.
        // let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 12453)).await?;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 12453)).await?;

        let local_addr = listener.local_addr()?;

        // Create server-side game session.
        let server = evoke::server::ServerSystem::builder()
            .with_descriptor::<Tank>()
            .with_descriptor::<TankState>()
            .with_descriptor::<TileMap>()
            .with_descriptor::<Global2>()
            .with_player::<RemoteTankPlayer>()
            .build(listener);

        // Set it to be executed in game loop.
        game.server = Some(server);

        tracing::info!("START: {}", local_addr);

        // Game configured. Run it.
        Ok(game)
    })
}

fn random_spawn_location(world: &mut World) -> Global2 {
    let maps_count = world
        .query_mut::<()>()
        .with::<TileMap>()
        .with::<TileSet>()
        .with::<Global2>()
        .into_iter()
        .count();

    let map_index = rand::random::<usize>() % maps_count;

    let (_, (map, set, global)) = world
        .query_mut::<(&TileMap, &TileSet, &Global2)>()
        .into_iter()
        .nth(map_index)
        .unwrap();

    let dim = map.dimensions();

    loop {
        let x = rand::random::<usize>() % dim.x;
        let y = rand::random::<usize>() % dim.y;

        let cell = map.cell_at(x, y);
        let tile = &set.tiles[cell];
        if tile.collider.is_none() {
            return Global2::new(global.iso * na::Translation2::from(map.cell_center(x, y)));
        }
    }
}
