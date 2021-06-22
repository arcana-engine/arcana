use {
    self::tank::*,
    arcana::{camera::Camera2, game2, na, Global2, Local2, Physics2, TimeSpan},
};

mod tank;

fn main() {
    game2(|mut game| async move {
        // game.loader
        //     .load_prefab::<TileMap>("".parse().unwrap(), &mut game.world);

        let tank1 = Tank::new(
            na::Vector2::new(1.0, 1.0),
            [0.8, 0.4, 0.1],
            "a6ba2179-d4d5-4a86-bd33-e82e97bb30aa".parse().unwrap(),
        );

        let tank1 = tank1.spawn(game.cx());

        // let tank2 = Tank::new(
        //     na::Vector2::new(1.0, 1.0),
        //     [0.4, 0.8, 0.1],
        //     "a6ba2179-d4d5-4a86-bd33-e82e97bb30aa".parse().unwrap(),
        // );

        // let tank2 = tank2.spawn(game.cx());

        // game.scheduler
        //     .add_fixed_system(Physics2::new(), TimeSpan::SECOND / 60);

        game.scheduler.add_system(tank::TankSystem);
        game.scheduler.add_system(tank::TankAnimationSystem::new());
        // game.scheduler.add_system(tank::BulletSystem);

        let camera = game.viewport.camera();

        game.world
            .get_mut::<Camera2>(camera)
            .unwrap()
            .set_scaley(0.2);

        game.control
            .assume_control(tank1, tank::TankController::main(), &mut game.world)?;

        // game.control
        //     .assume_control(tank2, tank::TankController::alt(), &mut game.world)?;

        Ok(game)
    })
}
