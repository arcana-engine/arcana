#![feature(allocator_api)]

use arcana::{
    assets::WithId, graphics::Texture, prelude::*, sprite::sprite_sheet::SpriteSheet,
    unfold::UnfoldResult,
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
                let mut q = cx.world.query::<&FooUnfoldSpawned>();
                for (e, spawned) in q.iter() {
                    let e = cx.world.entity(e).unwrap();
                    assert!(spawned.a.is_none() || e.has::<Texture>());
                    assert!(spawned.b.is_none() || e.has::<SpriteSheet<Texture>>());
                    if spawned.a.is_some() && spawned.b.is_some() {
                        tracing::error!("FOO LOADED");
                        foo_loaded = true;
                    }
                }
            }

            if !bar_loaded {
                let mut q = cx.world.query::<&BarUnfoldSpawned>();
                for (e, spawned) in q.iter() {
                    let e = cx.world.entity(e).unwrap();
                    if spawned.a.is_some() && spawned.b.is_some() {
                        assert!(e.has::<Texture>() && e.has::<SpriteSheet<Texture>>());
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
    b: TypedAssetId<SpriteSheet<Texture>>,
}

#[derive(Clone, Unfold)]
#[unfold(fn unfold_bar)]
pub struct Bar {
    #[unfold(asset: Texture)]
    a: AssetId,

    #[unfold(asset)]
    b: TypedAssetId<SpriteSheet<Texture>>,
}

fn unfold_bar(
    texture: &WithId<Texture>,
    sprite_sheet: &WithId<SpriteSheet<Texture>>,
    _: &mut Res,
) -> UnfoldResult<(Texture, SpriteSheet<Texture>)> {
    tracing::error!("UNFOLDING BAR");

    UnfoldResult {
        insert: (Clone::clone(texture), Clone::clone(&sprite_sheet)),
        spawn: Default::default(),
    }
}
