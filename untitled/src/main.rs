use poc::*;

fn main() {
    Game::new()
        // Register systems
        .with_system(camera::FreeCameraSystem)
        // Load something
        .with_prefab(assets::gltf::Gltf::new("sponza/Sponza.gltf".into()))
        // Setup controller
        .run(|game| {
            InputController::with_controlled(
                game.camera,
                game.world,
                camera::FreeCameraTranslator::new(),
            )
            .map_err(Into::into)
        });
}
