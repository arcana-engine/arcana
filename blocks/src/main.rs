use {
    arcana::*,
    rapier2d::{
        dynamics::RigidBodyBuilder,
        geometry::{Collider, ColliderBuilder},
        na,
    },
    std::time::Duration,
};

#[derive(Clone, Debug)]
struct Block;

impl Prefab for Block {
    type Loaded = assets::AssetResult<assets::ImageAsset>;
    type Fut = assets::AssetHandle<assets::ImageAsset>;

    fn load(&self, loader: &assets::Loader) -> Self::Fut {
        loader.load::<assets::ImageAsset>("cat.jpg")
    }

    fn spawn(
        mut cat: assets::AssetResult<assets::ImageAsset>,
        res: &mut Res,
        world: &mut hecs::World,
        graphics: &mut graphics::Graphics,
        entity: hecs::Entity,
    ) -> eyre::Result<()> {
        struct BlockCollider {
            cuboid: Collider,
        }

        let cuboid = res
            .with(|| BlockCollider {
                cuboid: ColliderBuilder::cuboid(0.015, 0.02)
                    .friction(0.5)
                    .restitution(0.8)
                    .build(),
            })
            .cuboid
            .clone();

        let mut physical_data = res.with(PhysicsData2::new);

        let body = physical_data.bodies.insert(
            RigidBodyBuilder::new_dynamic()
                .linvel(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5)
                .angvel(rand::random::<f32>() * 0.5 - 0.25)
                .build(),
        );

        let collider = physical_data
            .colliders
            .insert(cuboid, body, &mut physical_data.bodies);

        let cat = cat.get(graphics)?.unwrap().image.clone();

        let sampler = graphics.create_sampler(graphics::SamplerInfo::default())?;

        let _ = world.insert(
            entity,
            (
                graphics::Sprite {
                    pos: graphics::AABB {
                        left: -0.015,
                        right: 0.015,
                        top: -0.02,
                        bottom: 0.02,
                    },
                    ..graphics::Sprite::default()
                },
                graphics::Material {
                    albedo_coverage: Some(graphics::Texture {
                        image: cat,
                        sampler,
                    }),
                    ..Default::default()
                },
                Global2::new(
                    na::Translation2::new(
                        rand::random::<f32>() * 1.5 - 0.75,
                        rand::random::<f32>() * 1.5 - 0.75,
                    )
                    .into(),
                ),
                body,
            ),
        );

        Ok(())
    }
}

fn main() {
    game2(|mut game| async move {
        for _ in 0..1000 {
            game.loader.load_prefab(Block, &mut game.world);
        }

        let mut physical_data = game.res.with(PhysicsData2::new);

        let top = physical_data
            .bodies
            .insert(RigidBodyBuilder::new_static().build());
        let bottom = physical_data
            .bodies
            .insert(RigidBodyBuilder::new_static().build());
        let left = physical_data
            .bodies
            .insert(RigidBodyBuilder::new_static().build());
        let right = physical_data
            .bodies
            .insert(RigidBodyBuilder::new_static().build());

        physical_data.colliders.insert(
            ColliderBuilder::halfspace(na::UnitVector2::new_normalize(na::Vector2::new(0.0, 1.0)))
                .build(),
            top,
            &mut physical_data.bodies,
        );
        physical_data.colliders.insert(
            ColliderBuilder::halfspace(na::UnitVector2::new_normalize(na::Vector2::new(0.0, -1.0)))
                .build(),
            bottom,
            &mut physical_data.bodies,
        );
        physical_data.colliders.insert(
            ColliderBuilder::halfspace(na::UnitVector2::new_normalize(na::Vector2::new(1.0, 0.0)))
                .build(),
            left,
            &mut physical_data.bodies,
        );
        physical_data.colliders.insert(
            ColliderBuilder::halfspace(na::UnitVector2::new_normalize(na::Vector2::new(-1.0, 0.0)))
                .build(),
            right,
            &mut physical_data.bodies,
        );

        game.world
            .spawn((top, Global2::new(na::Translation2::new(0.0, -0.8).into())));
        game.world
            .spawn((bottom, Global2::new(na::Translation2::new(0.0, 0.8).into())));
        game.world
            .spawn((left, Global2::new(na::Translation2::new(-0.8, 0.0).into())));
        game.world
            .spawn((right, Global2::new(na::Translation2::new(0.8, 0.0).into())));

        game.scheduler
            .add_fixed_system(Physics2::new(), Duration::from_nanos(16_666_666));

        Ok(game)
    })
}
