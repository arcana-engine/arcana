use arcana::{
    assets, game2, graphics, hecs, na, AsyncTaskContext, Global2, SystemContext, TaskContext,
    TimeSpan,
};

#[derive(Clone, Debug)]
struct Bunny;

impl Bunny {
    fn spawn(self, cx: TaskContext<'_>) -> hecs::Entity {
        let cat = cx
            .loader
            .load::<assets::ImageAsset>(&"3368385b-dbfa-4d13-847f-df977e8c8dd4".parse().unwrap());

        let sampler = cx
            .graphics
            .create_sampler(graphics::SamplerInfo::default())
            .unwrap();

        let entity = cx.world.spawn((
            self,
            graphics::Sprite {
                world: graphics::Rect {
                    left: -0.015,
                    right: 0.015,
                    top: -0.02,
                    bottom: 0.02,
                },
                ..graphics::Sprite::default()
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

            let mut cx = AsyncTaskContext::new();
            let cx = cx.get();

            let cat = cat.get(cx.graphics).unwrap().clone().into_inner();

            let material = graphics::Material {
                albedo_coverage: Some(graphics::Texture {
                    image: cat,
                    sampler,
                }),
                ..Default::default()
            };

            let _ = cx.world.insert_one(entity, material);
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
            |mut cx: SystemContext<'_>| {
                if let Some(bunny) = cx.res.get::<BunnyCount>() {
                    println!("{} bunnies", bunny.count);
                }
            },
            TimeSpan::SECOND,
        );

        Ok(game)
    })
}
