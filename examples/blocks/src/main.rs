use arcana::{
    game::game2,
    graphics, na,
    physics2::{
        dynamics::RigidBodyBuilder,
        geometry::{Collider, ColliderBuilder},
        Physics2, PhysicsData2,
    },
    rect::Rect,
    scene::Global2,
    sprite::Sprite,
    system::SystemContext,
    TimeSpan,
};

#[derive(Clone, Debug)]
struct Block;

impl Block {
    fn spawn(cx: SystemContext<'_>) {
        struct BlockCollider {
            cuboid: Collider,
        }

        let cuboid = cx
            .res
            .with(|| BlockCollider {
                cuboid: ColliderBuilder::cuboid(0.005, 0.005)
                    .friction(0.5)
                    .restitution(0.8)
                    .build(),
            })
            .cuboid
            .clone();

        let physical_data = cx.res.with(PhysicsData2::new);

        let body = physical_data.bodies.insert(
            RigidBodyBuilder::new_dynamic()
                .linvel(na::Vector2::new(
                    rand::random::<f32>() - 0.5,
                    rand::random::<f32>() - 0.5,
                ))
                .angvel(rand::random::<f32>() * 0.5 - 0.25)
                .build(),
        );

        physical_data
            .colliders
            .insert_with_parent(cuboid, body, &mut physical_data.bodies);

        cx.world.spawn((
            Block,
            Sprite {
                world: Rect {
                    left: -0.005,
                    right: 0.005,
                    top: 0.005,
                    bottom: -0.005,
                },
                ..Sprite::default()
            },
            graphics::Material {
                albedo_factor: [0.3, 0.4, 0.5, 1.0],
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
        ));
    }
}

fn main() {
    game2(|mut game| async move {
        let physical_data = game.res.with(PhysicsData2::new);
        physical_data.gravity = na::Vector2::new(0.0, 1.0);

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

        physical_data.colliders.insert_with_parent(
            ColliderBuilder::halfspace(na::UnitVector2::new_normalize(na::Vector2::new(0.0, 1.0)))
                .build(),
            top,
            &mut physical_data.bodies,
        );
        physical_data.colliders.insert_with_parent(
            ColliderBuilder::halfspace(na::UnitVector2::new_normalize(na::Vector2::new(0.0, -1.0)))
                .build(),
            bottom,
            &mut physical_data.bodies,
        );
        physical_data.colliders.insert_with_parent(
            ColliderBuilder::halfspace(na::UnitVector2::new_normalize(na::Vector2::new(1.0, 0.0)))
                .build(),
            left,
            &mut physical_data.bodies,
        );
        physical_data.colliders.insert_with_parent(
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
            .add_fixed_system(Physics2::new(), TimeSpan::MILLISECOND * 20);

        game.scheduler
            .add_fixed_system(Block::spawn, TimeSpan::MILLISECOND * 10);

        game.scheduler.add_fixed_system(
            |cx: SystemContext<'_>| {
                let block_count = cx
                    .world
                    .query_mut::<()>()
                    .with::<Block>()
                    .into_iter()
                    .count();

                tracing::info!("{} blocks", block_count);
            },
            TimeSpan::SECOND,
        );

        Ok(game)
    })
}
