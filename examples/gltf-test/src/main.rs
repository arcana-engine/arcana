use arcana::*;

fn main() {
    game3(|mut game| async move {
        game.scheduler.add_system(camera::FreeCameraSystem);

        let mut object = game
            .loader
            .load::<assets::object::Object>(
                &"9054d348-31ad-428c-9a58-7bbc8a5907da".parse().unwrap(),
            )
            .await;

        dbg!(object.get(&mut game.graphics));

        Ok(game)
    })
}
