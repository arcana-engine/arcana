#![feature(allocator_api)]

use std::{
    net::Ipv4Addr,
    str::FromStr,
    sync::atomic::{AtomicU32, Ordering},
};

use arcana::{
    assets::tiles::{TileMap, TileMapComponent},
    game,
    hecs::{Entity, World},
    lifespan::LifeSpan,
    na,
    net::{
        server::{RemotePlayer, SelfDescriptor, ServerOwned, ServerSystem},
        PlayerId,
    },
    palette::{FromColor, Lch, Srgb},
    physics2::Physics2,
    Global2, Res, Spawner, TimeSpan,
};
use eyre::Context;
use tokio::net::TcpListener;

use tanks::*;
use uuid::Uuid;

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
    entity: Entity,
}

impl RemotePlayer for RemoteTankPlayer {
    type Command = tanks::TankCommand;
    type Info = ();

    fn accept(
        (): (),
        pid: PlayerId,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
    ) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        let tank = tanks::Tank::new(
            na::Vector2::new(1.0, 1.0),
            random_color(),
            "e12e16cd-9faf-4d61-b8cd-667ddecc823b".parse().unwrap(),
        );

        let entity = tank.spawn(
            &na::Translation2::new(rand::random(), rand::random()).into(),
            world,
            res,
            spawner,
        );
        world
            .insert(entity, (ServerOwned, pid))
            .expect("Just spawned");

        tracing::info!("Player's tank spawned");

        Ok(RemoteTankPlayer { entity })
    }

    #[inline(always)]
    fn disconnected(self, world: &mut World, _res: &mut Res, _spawner: &mut Spawner)
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
}

fn main() {
    game(|mut game| async move {
        let maps = [
            game.loader
                .load::<TileMap>(&Uuid::from_str("d5b2c243-bfff-4eb3-b10f-615faf210574").unwrap())
                .await
                .get(&mut ())
                .wrap_err("Failed to load tile map")?
                .clone(),
            game.loader
                .load::<TileMap>(&Uuid::from_str("5c6154dc-a98b-431d-8cf7-3627f9e5e6e0").unwrap())
                .await
                .get(&mut ())
                .wrap_err("Failed to load tile map")?
                .clone(),
            game.loader
                .load::<TileMap>(&Uuid::from_str("5c1fe447-bc12-496a-b713-9cf3a811b4d1").unwrap())
                .await
                .get(&mut ())
                .wrap_err("Failed to load tile map")?
                .clone(),
        ];

        // Bind listener for incoming connections.
        let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 12345)).await?;

        // Create server-side game session.
        let server = ServerSystem::builder()
            .with(Global2::descriptor())
            .with(TankState::descriptor())
            .with(Tank::descriptor())
            .with(TileMapComponent::descriptor())
            .build::<RemoteTankPlayer>(listener);

        // Set it to be executed in game loop.
        game.server = Some(server);

        tracing::info!("START");

        for i in -5..=5 {
            for j in -5..=5 {
                let index = rand::random::<usize>() % maps.len();
                let map = &maps[index];

                let offset =
                    na::Vector2::new(i as f32 - 0.5, j as f32 - 0.5).component_mul(&map.size());

                let entity = map.spawn(
                    &na::Isometry2::new(offset.into(), 0.0),
                    &mut game.world,
                    &mut game.res,
                )?;

                game.world
                    .insert_one(entity, ServerOwned)
                    .expect("Entity just spawned");
            }
        }

        game.scheduler
            .add_fixed_system(Physics2::new(), TimeSpan::MILLISECOND * 20);

        game.scheduler.add_ticking_system(tanks::TankSystem);
        game.scheduler.add_ticking_system(tanks::BulletSystem);

        // Game configured. Run it.
        Ok(game)
    })
}
