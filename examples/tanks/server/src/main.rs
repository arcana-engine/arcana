#![feature(allocator_api)]

use std::{
    net::Ipv4Addr,
    sync::atomic::{AtomicU32, Ordering},
};

use alkahest::{Seq, SeqUnpacked, Unpacked};
use arcana::{
    assets::tiles::{TileMap, TileMapComponent},
    bincode, game,
    hecs::{QueryOneError, World},
    na,
    net::{
        server::{RemotePlayer, ServerOwned, ServerSystem},
        PlayerId, ReplicaSerde,
    },
    palette::{FromColor, Hsl, Hsv, Lch, Srgb},
    physics2::Physics2,
    scoped_arena::Scope,
    timespan, CommandQueue, Global2, Res, Spawner, TimeSpan,
};
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

struct RemoteTankPlayer;

impl RemotePlayer for RemoteTankPlayer {
    type Command = tanks::TankCommand;
    type Info = ();
    type Input = Seq<(u8, u8)>;

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

        let entity = tank.spawn(world, res, spawner);
        world
            .insert(entity, (ServerOwned, pid))
            .expect("Just spawned");

        tracing::info!("Player's tank spawned");

        Ok(RemoteTankPlayer)
    }

    fn replicate_input(
        &mut self,
        input: SeqUnpacked<'_, (u8, u8)>,
        queue: &mut CommandQueue<tanks::TankCommand>,
        _scope: &Scope<'_>,
    ) {
        let commands = input.filter_map(|(d, v)| match d {
            0 => Some(TankCommand::Drive(i8::from_le_bytes([v]))),
            1 => Some(TankCommand::Rotate(i8::from_le_bytes([v]))),
            2 => Some(TankCommand::Fire(v > 0)),
            _ => None,
        });

        queue.enque(commands);
    }
}

type ReplicaSet = (Global2, TankState, Tank, TileMapComponent);

fn main() {
    game(|mut game| async move {
        // Bind listener for incoming connections.
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 12345)).await?;

        // Create server-side game session.
        let server = ServerSystem::new::<RemoteTankPlayer, ReplicaSet>(listener, timespan!(20ms));

        // Set it to be executed in game loop.
        game.server = Some(server);

        tracing::info!("START");

        let map = TileMap::load_and_spawn(
            &"a20280d4-a3e8-4a2a-8c51-381f021c11a7".parse().unwrap(),
            &na::Isometry2::identity(),
            &mut game.world,
            &mut game.res,
            &game.loader,
            &mut game.spawner,
        )
        .await?;

        game.world
            .insert_one(map, ServerOwned)
            .expect("Entity just spawned");

        game.scheduler
            .add_fixed_system(Physics2::new(), TimeSpan::MILLISECOND * 20);

        game.scheduler.add_ticking_system(tanks::TankSystem);
        game.scheduler.add_ticking_system(tanks::BulletSystem);

        // Game configured. Run it.
        Ok(game)
    })
}
