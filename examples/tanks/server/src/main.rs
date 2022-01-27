#![feature(allocator_api)]

use std::{
    net::Ipv4Addr,
    sync::atomic::{AtomicU32, Ordering},
};

use arcana::{
    assets::AssetId, evoke, hecs, na, palette::*, physics2::Physics2, prelude::*, tiles::TileMap,
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
    entity: hecs::Entity,
}

impl evoke::server::RemotePlayer for RemoteTankPlayer {
    type Input = Vec<tanks::TankCommand>;
    type Info = ();

    fn accept((): (), pid: evoke::PlayerId, world: &mut hecs::World) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        let entity = world.spawn((
            evoke::server::ServerOwned,
            pid,
            Global2::identity(),
            Tank {
                size: na::Vector2::new(1.0, 1.0),
                color: random_color(),
                sprite_sheet: AssetId::new(0x61cd051a6c24030d).unwrap(),
            },
            TankState::new(),
            CommandQueue::<TankCommand>::new(),
        ));

        tracing::info!("Player's tank spawned");

        Ok(RemoteTankPlayer { entity })
    }

    #[inline(always)]
    fn disconnected(self, world: &mut hecs::World)
    where
        Self: Sized,
    {
        tracing::error!("Set 5s TTL for tank of disconnected player");

        const KEEP_DISCONNECTED_FOR: TimeSpan = TimeSpan::from_seconds(5);

        match world.query_one_mut::<&mut LifeSpan>(self.entity) {
            Ok(lifespan) => lifespan.truncate(KEEP_DISCONNECTED_FOR),
            _ => {
                let _ = world.insert_one(self.entity, LifeSpan::new(KEEP_DISCONNECTED_FOR));
            }
        }
    }

    fn apply_input(
        &mut self,
        entity: hecs::Entity,
        world: &mut hecs::World,
        pack: Vec<tanks::TankCommand>,
    ) {
        if self.entity == entity {
            if let Ok(queue) = world.query_one_mut::<&mut CommandQueue<_>>(entity) {
                queue.enque(pack);
            }
        } else {
            tracing::error!("Player attempts to control not-owned entity");
        }
    }
}

fn main() {
    headless(|mut game| async move {
        let maps = [
            game.assets
                .load::<TileMap, _>("tanks-map1.json")
                .await
                .get()
                .wrap_err("Failed to load tile map")?
                .clone(),
            // game.assets
            //     .load::<TileMap, _>("tanks-map2.json")
            //     .await
            //     .get()
            //     .wrap_err("Failed to load tile map")?
            //     .clone(),
            // game.assets
            //     .load::<TileMap, _>("tanks-map3.json")
            //     .await
            //     .get()
            //     .wrap_err("Failed to load tile map")?
            //     .clone(),
        ];

        for i in -5..=5 {
            for j in -5..=5 {
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
        game.scheduler.add_ticking_system(tanks::TankSystem);
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
