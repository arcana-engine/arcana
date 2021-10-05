#![feature(allocator_api)]

use std::net::Ipv4Addr;

use alkahest::Unpacked;
use arcana::{
    net::server::{RemotePlayer, ServerSystem},
    timespan,
};
use tokio::net::TcpListener;

use {
    arcana::{assets::TileMap, game, na, physics2::Physics2, TimeSpan},
    tanks::*,
};

struct RemoteTankPlayer {}

impl RemotePlayer for RemoteTankPlayer {
    type Info = ();
    type Input = ();

    fn accept(
        info: Unpacked<'_, Self::Info>,
        res: &mut arcana::Res,
        world: &mut arcana::hecs::World,
    ) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        Ok(RemoteTankPlayer {})
    }

    fn apply_input(
        &mut self,
        input: Unpacked<'_, Self::Input>,
        res: &mut arcana::Res,
        world: &mut arcana::hecs::World,
    ) {
    }
}

fn main() {
    game(|mut game| async move {
        let listner = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;

        let server = ServerSystem::new::<RemoteTankPlayer, ()>(listner, timespan!(20ms));
        game.server = Some(server);

        // let _map = TileMap::load_and_spawn(
        //     &"a20280d4-a3e8-4a2a-8c51-381f021c11a7".parse().unwrap(),
        //     &na::Isometry2::identity(),
        //     &game.loader,
        //     &mut game.res,
        //     &mut game.world,
        //     &mut game.graphics,
        // )
        // .await?;

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

        Ok(game)
    })
}
