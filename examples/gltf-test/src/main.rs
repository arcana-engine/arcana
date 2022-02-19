use arcana::{assets, camera, game::game3};

fn main() {
    game3(|mut game| async move {
        game.scheduler.add_system(camera::FreeCamera3System);

        let model = game
            .assets
            .load::<assets::model::Model3d, _>("")
            .await
            .build(&mut game.graphics)?
            .clone();

        Ok(game)
    })
}
