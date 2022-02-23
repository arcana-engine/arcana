use arcana::{
    assets::model::Model, camera, control::EntityController, game::game3, na, prelude::Global3,
};

fn main() {
    game3(|mut game| async move {
        let model = game
            .assets
            .load::<Model, _>("helm/helm.gltf")
            .await
            .build(&mut game.graphics)?
            .clone();

        for primitive in model.primitives.iter() {
            if let Some(material) = primitive.material {
                let material = model.materials[material].clone();
                game.world
                    .spawn((primitive.mesh.clone(), material, Global3::identity()));
            }
        }

        let controller = EntityController::assume_control(
            camera::FreeCamera3Controller::new(),
            game.viewport.camera(),
            &mut game.world,
        )?;

        game.control.add_global_controller(controller);
        game.scheduler.add_system(camera::FreeCamera3System);

        let global3 = game
            .world
            .query_one_mut::<&mut Global3>(&game.viewport.camera())
            .unwrap();

        global3.iso.translation = na::Translation3::new(0.0, 0.0, 5.0);

        game.world
            .try_insert(&game.viewport.camera(), camera::FreeCamera3::new(1.0))
            .unwrap();

        Ok(game)
    })
}
