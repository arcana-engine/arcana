use arcana::{
    game::game2,
    graphics::{self, Texture},
    hecs, na,
    rect::Rect,
    scene::Global2,
    sprite::Sprite,
    system::SystemContext,
    task::{with_async_task_context, TaskContext},
    TimeSpan,
};

#[derive(Clone, Debug)]
struct Bunny;

impl Bunny {
    fn spawn(self, cx: TaskContext<'_>) -> hecs::Entity {
        let cat = cx.assets.load::<Texture, _>("bunny.png");

        let entity = cx.world.spawn((
            self,
            Sprite {
                world: Rect {
                    left: -0.015,
                    right: 0.015,
                    top: -0.02,
                    bottom: 0.02,
                },
                ..Sprite::default()
            },
            Global2::new(
                na::Translation2::new(
                    rand::random::<f32>() * 1.5 - 0.75,
                    rand::random::<f32>() * 1.5 - 0.75,
                )
                .into(),
            ),
        ));

        cx.spawner.spawn(async move {
            let mut cat = cat.await;

            with_async_task_context(|cx| {
                let cat = cat.build(cx.graphics).unwrap().clone();

                let material = graphics::Material {
                    albedo_coverage: Some(cat),
                    ..Default::default()
                };

                let _ = cx.world.insert_one(entity, material);
            });
            Ok(())
        });

        entity
    }
}

fn main() {
    game2(|mut game| async move {
        let start = 100000;

        for _ in 0..start {
            game.res.with(BunnyCount::default).count = start;
            Bunny.spawn(game.cx());
        }

        game.scheduler.add_system(move |cx: SystemContext<'_>| {
            for (_, global) in cx.world.query_mut::<&mut Global2>().with::<Bunny>() {
                let v = &mut global.iso.translation.vector;
                v.y -= cx.clock.delta.as_secs_f32();
                if v.y <= -0.75 {
                    v.y += 1.5;
                }
            }
        });

        #[derive(Default)]
        struct BunnyCount {
            count: u32,
        }

        game.scheduler.add_fixed_system(
            |mut cx: SystemContext<'_>| {
                cx.res.with(BunnyCount::default).count += 1;
                Bunny.spawn(cx.task());
            },
            TimeSpan::MILLISECOND,
        );

        game.scheduler.add_fixed_system(
            |cx: SystemContext<'_>| {
                if let Some(bunny) = cx.res.get::<BunnyCount>() {
                    println!("{} bunnies", bunny.count);
                }
            },
            TimeSpan::SECOND,
        );

        Ok(game)
    })
}
