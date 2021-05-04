use arcana::*;

fn main() {
    game3(|mut game| async move {
        game.scheduler.add_system(camera::FreeCameraSystem);
        game.loader.load_prefab(
            assets::gltf::prefab::Gltf::new("../assets/sponza/Sponza.gltf".into()),
            &mut game.world,
        );
        game.control
            .assume_control(
                game.viewport.camera(),
                camera::FreeCamera3Controller::new(),
                &mut game.world,
            )
            .unwrap();
        Ok(game)
    })
}
