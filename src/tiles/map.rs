use std::sync::Arc;

use goods::{Asset, AssetId};

#[cfg(feature = "client")]
use hecs::{Entity, World};

#[cfg(feature = "physics2d")]
use parry2d::shape::SharedShape;

#[cfg(feature = "physics2d")]
use rapier2d::prelude::{ColliderBuilder, RigidBodyBuilder, RigidBodyHandle};

#[cfg(feature = "physics2d")]
use crate::physics2::PhysicsData2;

use crate::{
    assets::WithId,
    resources::Res,
    unfold::{Unfold, UnfoldBundle, UnfoldResult},
};

use super::set::TileSet;

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize, Asset, Unfold)]
#[asset(name = "arcana.tilemap")]
#[unfold(fn unfold_tile_map)]
pub struct TileMap {
    #[unfold(asset: TileSet)]
    pub set: AssetId,
    pub cell_size: f32,
    pub width: usize,
    pub cells: Arc<[usize]>,
}

impl TileMap {
    pub fn size(&self) -> na::Vector2<f32> {
        if self.cells.len() == 0 {
            return na::Vector2::zeros();
        }
        let x = self.width;
        let y = ((self.cells.len() - 1) / self.width) + 1;
        self.cell_size * na::Vector2::new(x as f32, y as f32)
    }
}

fn unfold_tile_map(
    set: &WithId<TileSet>,
    cell_size: &f32,
    width: &usize,
    cells: &Arc<[usize]>,
    res: &mut Res,
) -> UnfoldResult<impl UnfoldBundle> {
    #[cfg(feature = "physics2d")]
    let body: RigidBodyHandle = {
        let mut compound = Vec::new();

        for (j, row) in cells.chunks(*width).enumerate() {
            for (i, &cell) in row.iter().enumerate() {
                let tile = match set.tiles.get(cell) {
                    None => {
                        tracing::error!("Missing tile '{}' in the tileset", cell);
                        continue;
                    }
                    Some(tile) => tile,
                };

                if let Some(collider) = tile.collider {
                    let tr = na::Translation2::new(i as f32 * cell_size, j as f32 * cell_size);

                    let shape = collider.shared_shape(*cell_size, res);
                    compound.push((tr.into(), shape));
                }
            }
        }

        let shape = SharedShape::compound(compound);

        let physics = res.with(PhysicsData2::new);
        let body = physics
            .bodies
            .insert(RigidBodyBuilder::new_static().build());

        physics.colliders.insert_with_parent(
            ColliderBuilder::new(shape).build(),
            body,
            &mut physics.bodies,
        );

        body
    };

    UnfoldResult::with_bundle((
        #[cfg(feature = "physics2d")]
        body,
        TileSet::clone(set),
    ))
}

#[cfg(any(feature = "client", feature = "server"))]
pub enum TileMapDescriptor {}

#[cfg(feature = "client")]
impl evoke::client::Descriptor for TileMapDescriptor {
    type Query = &'static mut TileMap;
    type Pack = TileMap;

    fn insert(pack: TileMap, entity: Entity, world: &mut World) {
        let _ = world.insert_one(entity, pack);
    }

    fn modify(pack: TileMap, item: &mut TileMap) {
        *item = pack;
    }

    fn remove(entity: Entity, world: &mut World) {
        let _ = world.remove_one::<TileMap>(entity);
    }
}

#[cfg(feature = "server")]
impl<'a> evoke::server::DescriptorPack<'a> for TileMapDescriptor {
    type Pack = &'a TileMap;
}

#[cfg(feature = "server")]
impl evoke::server::Descriptor for TileMapDescriptor {
    type Query = &'static TileMap;
    type History = TileMap;

    fn history(item: &TileMap) -> TileMap {
        item.clone()
    }

    fn replicate<'a>(
        item: &'a TileMap,
        history: Option<&TileMap>,
        _scope: &'a scoped_arena::Scope<'_>,
    ) -> evoke::server::Replicate<&'a TileMap> {
        match history {
            Some(history) if *history == *item => evoke::server::Replicate::Unmodified,
            _ => evoke::server::Replicate::Modified(item),
        }
    }
}
