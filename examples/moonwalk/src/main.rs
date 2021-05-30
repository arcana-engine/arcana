use {
    arcana::{
        assets::{SpriteSheet, TileMap},
        camera::Camera2,
        game2, na, AsyncTaskContext, Global2, Local2, Physics2, SystemContext,
    },
    std::time::Duration,
};

fn main() {
    game2(|mut game| async {
        let sprite_sheet = game
            .loader
            .load::<SpriteSheet>(&"ac62d0d9-4203-4173-933f-0839dff487b6".parse().unwrap());

        game.spawner.spawn(async move {
            let mut sprite_sheet = sprite_sheet.await;

            let mut acx = AsyncTaskContext::new();
            let cx = acx.get();

            let sprite_sheet = sprite_sheet.get(cx.graphics)?;

            Ok(())
        });

        Ok(game)
    });
}
