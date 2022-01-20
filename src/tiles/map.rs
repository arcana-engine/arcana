use std::sync::Arc;

use goods::{Asset, AssetId};

#[cfg(feature = "client")]
use hecs::{Entity, World};

#[cfg(feature = "physics2d")]
use parry2d::shape::SharedShape;
#[cfg(feature = "physics2d")]
use rapier2d::prelude::RigidBodyHandle;
#[cfg(feature = "physics2d")]
use rapier2d::prelude::{ColliderBuilder, RigidBodyBuilder};

#[cfg(feature = "physics2d")]
use crate::physics2::PhysicsData2;

use crate::{
    assets::AssetLoadCache,
    system::{System, SystemContext},
};

use super::set::TileSet;

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize, Asset)]
#[asset(name = "arcana.tile-map")]
pub struct TileMap {
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

pub(crate) struct TileMapSpawned {
    pub set_uuid: AssetId,
    pub set: TileSet,
}

pub struct TileMapSystem;

type TileMapSystemCache = AssetLoadCache<TileSet>;

impl System for TileMapSystem {
    fn name(&self) -> &str {
        "TileMapSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let cache = cx.res.with(TileMapSystemCache::new);

        let mut spawn = Vec::new_in(&*cx.scope);

        let query = cx
            .world
            .query_mut::<()>()
            .with::<TileMapSpawned>()
            .without::<TileMap>();

        let mut destruct = Vec::new_in(&*cx.scope);
        destruct.extend(query.into_iter().map(|(e, ())| e));

        for e in destruct {
            let _ = cx.world.remove_one::<TileMapSpawned>(e);

            #[cfg(feature = "physics2d")]
            let _ = cx.world.remove_one::<RigidBodyHandle>(e);
        }

        let query = cx
            .world
            .query_mut::<(&TileMap, Option<&mut TileMapSpawned>)>();

        for (entity, (map, spawned)) in query {
            match &spawned {
                Some(spawned) if spawned.set_uuid == map.set => {
                    continue;
                }
                _ => cache.load(map.set, cx.loader),
            }

            match spawned {
                None => {
                    if let Some(set) = cache.get_ready(map.set) {
                        spawn.push((entity, set.clone(), map.clone()));
                    }
                }
                Some(spawned) => {
                    if spawned.set_uuid != map.set {
                        if let Some(set) = cache.get_ready(map.set) {
                            spawn.push((entity, set.clone(), map.clone()));
                        }
                    } else {
                        spawn.push((entity, spawned.set.clone(), map.clone()));
                    }
                }
            }
        }

        #[cfg(feature = "graphics")]
        cache.ensure_task(cx.spawner, |cx| cx.graphics);
        #[cfg(not(feature = "visible"))]
        cache.ensure_task(cx.spawner, |cx| cx.world);

        for (entity, set, map) in spawn {
            #[cfg(feature = "physics2d")]
            {
                let mut compound = Vec::new();

                for (j, row) in map.cells.chunks(map.width).enumerate() {
                    for (i, &cell) in row.iter().enumerate() {
                        let tile = match set.tiles.get(cell) {
                            None => {
                                return Err(eyre::eyre!("Missing tile '{}' in the tileset", cell));
                            }
                            Some(tile) => tile,
                        };

                        if let Some(collider) = tile.collider {
                            let tr = na::Translation2::new(
                                i as f32 * map.cell_size,
                                j as f32 * map.cell_size,
                            );

                            let shape = collider.shared_shape(map.cell_size, cx.res);
                            compound.push((tr.into(), shape));
                        }
                    }
                }

                let shape = SharedShape::compound(compound);

                let physics = cx.res.with(PhysicsData2::new);
                let body = physics
                    .bodies
                    .insert(RigidBodyBuilder::new_static().build());

                physics.colliders.insert_with_parent(
                    ColliderBuilder::new(shape).build(),
                    body,
                    &mut physics.bodies,
                );

                let _ = cx.world.insert_one(entity, body);
            }

            let _ = cx.world.insert(
                entity,
                (TileMapSpawned {
                    set,
                    set_uuid: map.set,
                },),
            );
        }

        let cache = cx.res.get_mut::<TileMapSystemCache>().unwrap();
        cache.clear_ready();

        Ok(())
    }
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
        let _ = world.remove_one::<TileMapSpawned>(entity);
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
