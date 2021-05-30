use {
    arcana::{
        assets::tiles::TileMap, camera::Camera2, game2, na, Global2, Local2, Physics2,
        SystemContext,
    },
    std::time::Duration,
};

mod tank;

fn main() {
    game2(|mut game| async move {
        // game.loader
        //     .load_prefab::<TileMap>("".parse().unwrap(), &mut game.world);

        // let tank = game.loader.load_prefab(
        //     tank::Tank::new(
        //         na::Vector2::new(0.9, 0.9),
        //         [0.2, 0.9, 0.2],
        //         "tank-1.json".into(),
        //     ),
        //     &mut game.world,
        // );

        // let tank2 = game.loader.load_prefab(
        //     tank::Tank::new(
        //         "tank-1.json".into(),
        //         na::Vector2::new(0.9, 0.9),
        //         [0.9, 0.2, 0.2],
        //     ),
        //     &mut game.world,
        // );

        game.scheduler
            .add_fixed_system(Physics2::new(), Duration::from_nanos(16_666_666));

        // game.scheduler.add_system(Physics2::new());
        game.scheduler.add_system(tank::TankSystem);
        game.scheduler.add_system(tank::SpriteAnimationSystem);
        game.scheduler.add_system(tank::BulletSystem);

        let camera = game.viewport.camera();

        game.world
            .get_mut::<Camera2>(camera)
            .unwrap()
            .set_scaley(0.2);

        // game.scheduler.add_system(move |cx: SystemContext<'_>| {
        //     if let Ok(global) = cx.world.get::<Global2>(tank) {
        //         let target = global.iso.translation.vector;

        //         if let Ok(mut global) = cx.world.get_mut::<Global2>(camera) {
        //             global.iso.translation.vector = global.iso.translation.vector.lerp(
        //                 &target,
        //                 (cx.clock.delta.as_secs_f32() * 5.0).clamp(0.0, 1.0),
        //             );
        //         }
        //     }
        // });

        // game.control
        //     .assume_control(tank, tank::TankController::main(), &mut game.world)?;

        // game.control
        //     .assume_control(tank2, tank::TankController::alt(), &mut game.world)?;

        Ok(game)
    })
}
