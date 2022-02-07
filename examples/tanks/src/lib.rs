#![feature(allocator_api)]

use arcana::{
    assets::{Asset, AssetId},
    edict::bundle::Bundle,
    lifespan::LifeSpan,
    na,
    physics2::{prelude::*, *},
    prelude::*,
    unfold::UnfoldResult,
};

#[cfg(feature = "graphics")]
use arcana::{
    assets::WithId,
    graphics::{Material, Texture},
    rect::Rect,
    sprite::*,
};

pub struct Bullet;

pub struct BulletCollider(pub Collider);

impl BulletCollider {
    pub fn new() -> Self {
        BulletCollider(
            ColliderBuilder::ball(0.1)
                .active_events(ActiveEvents::CONTACT_EVENTS)
                .build(),
        )
    }
}

#[derive(Debug)]
pub enum TankAnimTransitionRule {
    Moving,
    Idle,
    Broken,
    AnimationComplete,
}

#[cfg(feature = "graphics")]
impl AnimTransitionRule<TankState> for TankAnimTransitionRule {
    fn matches(&self, state: &TankState, info: &CurrentAnimInfo) -> bool {
        match self {
            Self::Moving => (state.drive != 0 || state.rotate != 0) && state.alive,
            Self::Idle => state.drive == 0 && state.rotate == 0 && state.alive,
            Self::Broken => !state.alive,
            Self::AnimationComplete => info.is_complete(),
        }
    }
}

#[cfg(feature = "graphics")]
fn tank_graph_animation(sheet: &SpriteSheetMeta) -> SpriteGraphAnimation<TankAnimTransitionRule> {
    SpriteGraphAnimation::new(
        0,
        sheet,
        vec![
            (TankAnimTransitionRule::AnimationComplete, vec![0], 0),
            (TankAnimTransitionRule::AnimationComplete, vec![1], 1),
            (TankAnimTransitionRule::Moving, vec![0, 2], 1),
            (TankAnimTransitionRule::Broken, vec![0, 1], 2),
            (TankAnimTransitionRule::Idle, vec![1, 2], 0),
        ],
    )
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TankState {
    pub drive: i8,
    pub rotate: i8,
    pub fire: bool,
    pub alive: bool,
}

impl TankState {
    pub fn new() -> Self {
        TankState {
            drive: 0,
            rotate: 0,
            fire: false,
            alive: true,
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum TankCommand {
    Drive(i8),
    Rotate(i8),
    Fire,
}

pub struct BulletSystem;

impl System for BulletSystem {
    fn name(&self) -> &str {
        "BulletSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        let mut despawn = Vec::new_in(&*cx.scope);

        for (e, queue) in cx.world.query_mut::<&mut ContactQueue2>().with::<Bullet>() {
            if queue.drain_contacts_started().count() > 0 {
                despawn.push(e);
            }
            queue.drain_contacts_stopped();
        }

        for e in despawn {
            #[cfg(feature = "graphics")]
            if let Ok(iso) = cx.world.query_one::<&Global2>(&e).map(|g| g.iso) {
                cx.world.spawn((
                    Global2::new(iso),
                    Sprite {
                        world: Rect {
                            left: -0.2,
                            right: 0.2,
                            top: -0.2,
                            bottom: 0.2,
                        },
                        src: Rect::ONE_QUAD,
                        tex: Rect::ONE_QUAD,
                        layer: 0,
                    },
                    Material {
                        albedo_factor: [1.0, 0.3, 0.1, 1.0],
                        ..Default::default()
                    },
                    LifeSpan::new(TimeSpan::SECOND * 5),
                ));
            }
            let _ = cx.world.despawn(&e);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Asset)]
#[asset(name = "tank")]
#[derive(Unfold)]
#[unfold(fn unfold_tank)]
pub struct Tank {
    pub size: na::Vector2<f32>,
    pub color: [f32; 3],

    #[cfg_attr(feature = "graphics", unfold(asset: SpriteSheet<Texture>))]
    pub sprite_sheet: AssetId,
}

#[allow(unused_variables)]
fn unfold_tank(
    size: &na::Vector2<f32>,
    color: &[f32; 3],
    #[cfg(feature = "graphics")] sprite_sheet: &WithId<SpriteSheet<Texture>>,
    #[cfg(not(feature = "graphics"))] _sprite_sheet: &AssetId,
    res: &mut Res,
) -> UnfoldResult<impl Bundle> {
    let hs = size / 2.0;
    let physics = res.with(PhysicsData2::new);

    let body = physics.bodies.insert(
        RigidBodyBuilder::new_dynamic()
            .linear_damping(0.3)
            .angular_damping(0.3)
            .build(),
    );

    physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(hs.x * 0.625, hs.y * 0.6875)
            .active_events(ActiveEvents::CONTACT_EVENTS)
            .build(),
        body,
        &mut physics.bodies,
    );

    UnfoldResult::with_bundle((
        body,
        ContactQueue2::new(),
        #[cfg(feature = "graphics")]
        Sprite {
            world: Rect {
                left: -hs.x,
                right: hs.x,
                top: -hs.y,
                bottom: hs.y,
            },
            src: Rect::ONE_QUAD,
            tex: Rect::ONE_QUAD,
            layer: 1,
        },
        #[cfg(feature = "graphics")]
        Material {
            albedo_coverage: Some(sprite_sheet.texture.clone()),
            albedo_factor: [color[0], color[1], color[2], 1.0],
            ..Default::default()
        },
        #[cfg(feature = "graphics")]
        tank_graph_animation(&sprite_sheet),
    ))
}

#[cfg(feature = "graphics")]
pub type TankAnimationSystem = SpriteGraphAnimationSystem<TankState, TankAnimTransitionRule>;
