use poc::*;

fn main() {
    game(|mut game| async move {
        game.scheduler.add_system(camera::FreeCameraSystem);
        game.loader.load_prefab(
            assets::gltf::Gltf::new("sponza/Sponza.gltf".into()),
            &mut game.world,
        );
        game.control
            .assume_control(
                game.viewport.camera(),
                camera::FreeCameraController::new(),
                &mut game.world,
            )
            .unwrap();
        Ok(game)
    })
}
