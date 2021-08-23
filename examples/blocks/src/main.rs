use arcana::{
    game2, graphics, na,
    physics2::{
        dynamics::RigidBodyBuilder,
        geometry::{Collider, ColliderBuilder},
        Physics2, PhysicsData2,
    },
    Global2, TaskContext, TimeSpan,
};

#[derive(Clone, Debug)]
struct Block;

impl Block {
    fn spawn(self, cx: TaskContext<'_>) -> hecs::Entity {
        struct BlockCollider {
            cuboid: Collider,
        }

        let cuboid = cx
            .res
            .with(|| BlockCollider {
                cuboid: ColliderBuilder::cuboid(0.02, 0.02)
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

        let collider =
            physical_data
                .colliders
                .insert_with_parent(cuboid, body, &mut physical_data.bodies);

        let sampler = cx
            .graphics
            .create_sampler(graphics::SamplerInfo::default())
            .unwrap();

        let entity = cx.world.spawn((
            self,
            graphics::Sprite {
                world: graphics::Rect {
                    left: -0.02,
                    right: 0.02,
                    top: -0.02,
                    bottom: 0.02,
                },
                ..graphics::Sprite::default()
            },
            graphics::Material {
                albedo_factor: [0.3.into(), 0.4.into(), 0.5.into()],
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

        entity
    }
}

fn main() {
    game2(|mut game| async move {
        for _ in 0..1000 {
            Block.spawn(game.cx());
        }

        let physical_data = game.res.with(PhysicsData2::new);

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

        Ok(game)
    })
}
