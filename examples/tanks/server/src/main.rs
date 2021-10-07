#![feature(allocator_api)]

use std::net::Ipv4Addr;

use alkahest::{Bytes, Unpacked};
use arcana::{
    assets::tiles::{TileMap, TileMapComponent},
    bincode, game,
    hecs::QueryOneError,
    na,
    net::{
        server::{RemotePlayer, ServerOwned, ServerSystem},
        PlayerId, ReplicaPrefabSerde, ReplicaSerde,
    },
    physics2::Physics2,
    scoped_arena::Scope,
    timespan, CommandQueue, Global2, TimeSpan,
};
use tokio::net::TcpListener;

use tanks::*;

struct RemoteTankPlayer {}

impl RemotePlayer for RemoteTankPlayer {
    type Command = tanks::TankCommand;
    type Info = ();
    type Input = Bytes;

    fn accept(
        info: Unpacked<'_, Self::Info>,
        pid: PlayerId,
        res: &mut arcana::Res,
        world: &mut arcana::hecs::World,
    ) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        Ok(RemoteTankPlayer {})
    }

    fn replicate_input(
        &mut self,
        input: &[u8],
        queue: &mut CommandQueue<tanks::TankCommand>,
        scope: &Scope<'_>,
    ) {
        let commands: Vec<tanks::TankCommand> =
            bincode::deserialize_from(input).expect("Failed to deserialize command");
        queue.enque(commands);
    }
}

type ReplicaSet = (ReplicaPrefabSerde<TileMapComponent>, Global2);

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

        game.world.insert_one(map, ServerOwned);

        // {
        //     let tank1 = Tank::new(
        //         na::Vector2::new(1.0, 1.0),
        //         [0.8, 0.4, 0.1],
        //         "e12e16cd-9faf-4d61-b8cd-667ddecc823b".parse().unwrap(),
        //     );
        // }

        // game.scheduler
        //     .add_fixed_system(Physics2::new(), TimeSpan::MILLISECOND * 20);

        // game.scheduler.add_system(tanks::TankSystem);
        // game.scheduler.add_system(tanks::TankAnimationSystem::new());
        // game.scheduler.add_system(tanks::BulletSystem);

        // Game configured. Run it.
        Ok(game)
    })
}
