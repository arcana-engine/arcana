#![feature(allocator_api)]

use arcana::{
    assets::WithId,
    edict::bundle::Bundle,
    game::{game2, Exit},
    graphics::Texture,
    resources::Res,
    sprite::SpriteSheet,
    system::SystemContext,
    unfold::{Unfold, UnfoldResult},
};

use goods::*;

fn main() {
    game2(|mut game| async move {
        Foo::schedule_unfold_system(&mut game.scheduler);
        Bar::schedule_unfold_system(&mut game.scheduler);

        game.world.spawn((Foo {
            a: AssetId::new(0x5321e2914afca30d).unwrap(),
            b: TypedAssetId::new(0x61cd051a6c24030d).unwrap(),
        },));

        game.world.spawn((Bar {
            a: AssetId::new(0x5321e2914afca30d).unwrap(),
            b: TypedAssetId::new(0x61cd051a6c24030d).unwrap(),
        },));

        let mut foo_loaded = false;
        let mut bar_loaded = false;

        game.scheduler.add_system(move |cx: SystemContext<'_>| {
            if !foo_loaded {
                let q = cx.world.query::<&FooUnfoldSpawned>();
                for (e, spawned) in q.iter() {
                    assert!(matches!(cx.world.has_component::<Texture>(&e), Ok(true)));
                    assert!(matches!(
                        cx.world.has_component::<SpriteSheet>(&e),
                        Ok(true)
                    ));
                    if spawned.a.is_some() && spawned.b.is_some() {
                        tracing::error!("FOO LOADED");
                        foo_loaded = true;
                    }
                }
            }

            if !bar_loaded {
                let q = cx.world.query::<&BarUnfoldSpawned>();
                for (e, spawned) in q.iter() {
                    if spawned.a.is_some() && spawned.b.is_some() {
                        assert!(matches!(cx.world.has_component::<Texture>(&e), Ok(true)));
                        assert!(matches!(
                            cx.world.has_component::<SpriteSheet>(&e),
                            Ok(true)
                        ));
                        tracing::error!("BAR LOADED");
                        bar_loaded = true;
                    }
                }
            }

            if foo_loaded && bar_loaded {
                cx.res.insert(Exit);
            }
        });

        Ok(game)
    });
}

#[derive(Unfold)]
pub struct Foo {
    #[unfold(asset: Texture)]
    a: AssetId,

    #[unfold(asset)]
    b: TypedAssetId<SpriteSheet>,
}

#[derive(Clone, Unfold)]
#[unfold(fn unfold_bar)]
pub struct Bar {
    #[unfold(asset: Texture)]
    a: AssetId,

    #[unfold(asset)]
    b: TypedAssetId<SpriteSheet>,
}

fn unfold_bar(
    texture: &WithId<Texture>,
    sprite_sheet: &WithId<SpriteSheet>,
    _: &mut Res,
) -> UnfoldResult<impl Bundle> {
    tracing::error!("UNFOLDING BAR");

    UnfoldResult {
        insert: (Clone::clone(texture), Clone::clone(sprite_sheet)),
        spawn: Default::default(),
    }
}
