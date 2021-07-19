use {
    self::tank::*,
    arcana::{assets::TileMap, camera::Camera2, game2, na, EntityController, Physics2, TimeSpan},
};

mod tank;

fn main() {
    game2(|mut game| async move {
        let _map = TileMap::load_and_spawn(
            &"55f1799c-303a-480f-b156-5b51fab18ae5".parse().unwrap(),
            &game.loader,
            &mut game.res,
            &mut game.world,
            &mut game.graphics,
        )
        .await?;

        let tank1 = Tank::new(
            na::Vector2::new(1.0, 1.0),
            [0.8, 0.4, 0.1],
            "a6ba2179-d4d5-4a86-bd33-e82e97bb30aa".parse().unwrap(),
        );

        let tank2 = Tank::new(
            na::Vector2::new(1.0, 1.0),
            [0.1, 0.4, 0.8],
            "a6ba2179-d4d5-4a86-bd33-e82e97bb30aa".parse().unwrap(),
        );

        let tank1 = tank1.spawn(game.cx());
        let tank2 = tank2.spawn(game.cx());

        game.scheduler
            .add_fixed_system(Physics2::new(), TimeSpan::MILLISECOND * 20);

        game.scheduler.add_system(tank::TankSystem);
        game.scheduler.add_system(tank::TankAnimationSystem::new());
        game.scheduler.add_system(tank::BulletSystem);

        let camera = game.viewport.camera();

        game.world
            .get_mut::<Camera2>(camera)
            .unwrap()
            .set_scaley(0.2);

        let controller1 = EntityController::assume_control(
            tank::TankComander::main(),
            10,
            tank1,
            &mut game.world,
        )?;

        let controller2 = EntityController::assume_control(
            tank::TankComander::alt(),
            10,
            tank2,
            &mut game.world,
        )?;

        game.control.add_global_controller(controller1);
        game.control.add_global_controller(controller2);

        Ok(game)
    })
}
