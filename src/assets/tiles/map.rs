use std::{mem::replace, sync::Arc};

use goods::{Asset, Loader, Uuid};
use hecs::{Entity, World};
use rapier2d::prelude::{ColliderBuilder, RigidBodyBuilder};

use crate::{
    assets::{AssetLoadSystemCache, WithUuid},
    physics2::PhysicsData2,
    Global2, Local2, Res, System,
};

#[cfg(feature = "visible")]
use crate::graphics::{Graphics, Material, Rect, Sprite};

use super::set::TileSet;

#[derive(Clone, Debug, Asset)]
pub struct TileMap {
    #[external]
    pub set: WithUuid<TileSet>,
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

    pub fn spawn(
        &self,
        origin: &na::Isometry2<f32>,
        world: &mut World,
        res: &mut Res,
    ) -> eyre::Result<Entity> {
        let entity = world.spawn((self.clone(), Global2::new(*origin)));

        let tiles = spawn_tiles(
            Some(entity),
            origin,
            &self.set,
            &self.cells,
            self.cell_size,
            self.width,
            world,
            res,
        )?;

        Ok(entity)
    }

    pub fn spawn_tiles(
        &self,
        origin: &na::Isometry2<f32>,
        world: &mut World,
        res: &mut Res,
    ) -> eyre::Result<()> {
        spawn_tiles(
            None,
            origin,
            &self.set,
            &self.cells,
            self.cell_size,
            self.width,
            world,
            res,
        )?;

        Ok(())
    }

    pub async fn load_and_spawn(
        uuid: &Uuid,
        origin: &na::Isometry2<f32>,
        world: &mut World,
        res: &mut Res,
        loader: &Loader,
        #[cfg(feature = "visible")] graphics: &mut Graphics,
    ) -> eyre::Result<Entity> {
        let mut map = loader.load::<Self>(uuid).await;
        #[cfg(feature = "visible")]
        let map = map.get(graphics)?;
        #[cfg(not(feature = "visible"))]
        let map = map.get(&mut ())?;

        map.spawn(origin, world, res)
    }

    pub async fn load_and_spawn_individual_tiles(
        uuid: &Uuid,
        origin: &na::Isometry2<f32>,
        world: &mut World,
        res: &mut Res,
        loader: &Loader,
        #[cfg(feature = "visible")] graphics: &mut Graphics,
    ) -> eyre::Result<()> {
        let mut map = loader.load::<Self>(uuid).await;
        #[cfg(feature = "visible")]
        let map = map.get(graphics)?;
        #[cfg(not(feature = "visible"))]
        let map = map.get(&mut ())?;

        map.spawn_tiles(origin, world, res)
    }
}

fn spawn_tiles(
    map: Option<Entity>,
    origin: &na::Isometry2<f32>,
    set: &TileSet,
    cells: &[usize],
    cell_size: f32,
    width: usize,
    world: &mut World,
    res: &mut Res,
) -> eyre::Result<Vec<Entity>> {
    let mut tiles = Vec::new();
    let hc = cell_size * 0.5;

    // let mut compound = Vec::new();

    for (j, row) in cells.chunks(width).enumerate() {
        for (i, &cell) in row.iter().enumerate() {
            let tile = match set.tiles.get(cell) {
                None => {
                    return Err(eyre::eyre!("Missing tile '{}' in the tileset", cell));
                }
                Some(tile) => tile,
            };

            #[cfg(feature = "visible")]
            let albedo_coverage = tile.texture.clone();

            let local_tr = na::Translation2::new(i as f32 * cell_size, j as f32 * cell_size);

            match tile.collider {
                None => {
                    let e = world.spawn((
                        Global2::new(origin * local_tr),
                        #[cfg(feature = "visible")]
                        Sprite {
                            world: Rect {
                                left: -hc,
                                right: hc,
                                top: -hc,
                                bottom: hc,
                            },
                            src: Rect::ONE_QUAD,
                            tex: Rect::ONE_QUAD,
                            layer: 10,
                        },
                        #[cfg(feature = "visible")]
                        Material {
                            albedo_coverage,
                            ..Default::default()
                        },
                    ));

                    if let Some(map) = map {
                        world
                            .insert_one(e, Local2::new(map, local_tr.into()))
                            .unwrap();
                    }

                    tiles.push(e);
                }
                Some(collider) => match map {
                    None => {
                        let iso = origin * local_tr;
                        let shape = collider.shared_shape(cell_size, res);

                        let physics = res.with(PhysicsData2::new);
                        let body = physics
                            .bodies
                            .insert(RigidBodyBuilder::new_static().position(iso).build());

                        physics.colliders.insert_with_parent(
                            ColliderBuilder::new(shape).build(),
                            body,
                            &mut physics.bodies,
                        );

                        let e = world.spawn((
                            Global2::new(iso),
                            #[cfg(feature = "visible")]
                            Sprite {
                                world: Rect {
                                    left: -hc,
                                    right: hc,
                                    top: -hc,
                                    bottom: hc,
                                },
                                src: Rect::ONE_QUAD,
                                tex: Rect::ONE_QUAD,
                                layer: 10,
                            },
                            #[cfg(feature = "visible")]
                            Material {
                                albedo_coverage,
                                ..Default::default()
                            },
                            body,
                        ));

                        tiles.push(e);
                    }
                    Some(map) => {
                        let iso = origin * local_tr;
                        let local_iso = local_tr.into();

                        let shape = collider.shared_shape(cell_size, res);
                        // compound.push((local_iso, shape));

                        let physics = res.with(PhysicsData2::new);
                        let body = physics
                            .bodies
                            .insert(RigidBodyBuilder::new_static().position(iso).build());

                        physics.colliders.insert_with_parent(
                            ColliderBuilder::new(shape).build(),
                            body,
                            &mut physics.bodies,
                        );

                        let e = world.spawn((
                            Local2::new(map, local_iso),
                            Global2::new(iso),
                            #[cfg(feature = "visible")]
                            Sprite {
                                world: Rect {
                                    left: -hc,
                                    right: hc,
                                    top: -hc,
                                    bottom: hc,
                                },
                                src: Rect::ONE_QUAD,
                                tex: Rect::ONE_QUAD,
                                layer: 10,
                            },
                            #[cfg(feature = "visible")]
                            Material {
                                albedo_coverage,
                                ..Default::default()
                            },
                        ));

                        tiles.push(e);
                    }
                },
            }
        }
    }

    // if let Some(map) = map {
    //     let shape = SharedShape::compound(compound);

    //     let physics = res.with(PhysicsData2::new);
    //     let body = physics
    //         .bodies
    //         .insert(RigidBodyBuilder::new_static().build());

    //     physics.colliders.insert_with_parent(
    //         ColliderBuilder::new(shape).build(),
    //         body,
    //         &mut physics.bodies,
    //     );

    //     world.insert_one(map, body);
    // }

    Ok(tiles)
}

#[cfg(any(feature = "client", feature = "server"))]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct TileMapReplica {
    set: Uuid,
    cell_size: f32,
    width: usize,
    cells: Arc<[usize]>,
}

impl TileMapReplica {
    fn from_map(map: &TileMap) -> Self {
        TileMapReplica {
            set: WithUuid::uuid(&map.set),
            cell_size: map.cell_size,
            width: map.width,
            cells: map.cells.clone(),
        }
    }

    fn equivalent(&self, map: &TileMap) -> bool {
        self.set == WithUuid::uuid(&map.set)
            && self.cell_size == map.cell_size
            && self.width == map.width
            && self.cells == map.cells
    }
}

pub struct TileMapTiles {
    array: Vec<Entity>,
}

pub struct TileMapReplicaSystem;

type TileMapReplicaSystemCache = AssetLoadSystemCache<TileSet>;

impl System for TileMapReplicaSystem {
    fn name(&self) -> &str {
        "TileMapReplicaSystem"
    }

    fn run(&mut self, cx: crate::SystemContext<'_>) -> eyre::Result<()> {
        let cache = cx.res.with(TileMapReplicaSystemCache::new);

        let mut spawn = Vec::new_in(&*cx.scope);
        let mut remove_replica = Vec::new_in(&*cx.scope);

        let query = cx.world.query_mut::<(
            &Global2,
            &TileMapReplica,
            Option<&mut TileMap>,
            Option<&mut TileMapTiles>,
        )>();

        for (entity, (global, replica, map, tiles)) in query {
            match &map {
                Some(old) if replica.equivalent(old) => {
                    remove_replica.push(entity);
                    continue;
                }
                _ => {}
            }

            match &map {
                Some(old) if WithUuid::uuid(&old.set) == replica.set => {}
                _ => {
                    cache.ensure_load(replica.set, cx.loader);
                }
            }

            match map {
                None => {
                    if let Some(set) = cache.get_ready(&replica.set) {
                        spawn.push((entity, set.clone(), global.iso, Vec::new()));
                    }
                }
                Some(map) => {
                    if WithUuid::uuid(&map.set) != replica.set {
                        if let Some(set) = cache.get_ready(&replica.set) {
                            let tiles = match tiles {
                                Some(tiles) => replace(&mut tiles.array, Vec::new()),
                                None => Vec::new(),
                            };
                            spawn.push((entity, set.clone(), global.iso, tiles));
                        }
                    } else {
                        let tiles = match tiles {
                            Some(tiles) => replace(&mut tiles.array, Vec::new()),
                            None => Vec::new(),
                        };
                        spawn.push((entity, (&*map.set).clone(), global.iso, tiles));
                    }
                }
            }
        }

        #[cfg(feature = "visible")]
        cache.ensure_task(cx.spawner, |cx| cx.graphics);
        #[cfg(not(feature = "visible"))]
        cache.ensure_task(cx.spawner, |cx| cx.world);

        for (entity, set, iso, tiles) in spawn {
            for e in tiles {
                let _ = cx.world.despawn(e);
            }

            let replica = cx.world.remove_one::<TileMapReplica>(entity).unwrap();

            let tiles = spawn_tiles(
                Some(entity),
                &iso,
                &set,
                &replica.cells,
                replica.cell_size,
                replica.width,
                cx.world,
                cx.res,
            )
            .unwrap();

            let _ = cx.world.insert(
                entity,
                (
                    TileMap {
                        set: WithUuid::new(set, replica.set),
                        cell_size: replica.cell_size,
                        width: replica.width,
                        cells: replica.cells.clone(),
                    },
                    TileMapTiles { array: tiles },
                ),
            );
        }

        for entity in remove_replica {
            let _ = cx.world.remove_one::<TileMapReplica>(entity);
        }

        let cache = cx.res.get_mut::<TileMapReplicaSystemCache>().unwrap();
        cache.clear_ready();

        Ok(())
    }
}

#[cfg(any(feature = "client", feature = "server"))]
pub enum TileMapDescriptor {}

#[cfg(feature = "client")]
impl evoke::client::Descriptor for TileMapDescriptor {
    type Query = &'static mut TileMapReplica;
    type Pack = TileMapReplica;

    fn insert(pack: TileMapReplica, entity: Entity, world: &mut World) {
        let _ = world.insert_one(entity, pack);
    }

    fn modify(pack: TileMapReplica, item: &mut TileMapReplica) {
        *item = pack;
    }

    fn remove(entity: Entity, world: &mut World) {
        let _ = world.remove_one::<TileMapReplica>(entity);
        let _ = world.remove_one::<TileMap>(entity);
        if let Ok(tiles) = world.remove_one::<TileMapTiles>(entity) {
            for e in tiles.array {
                let _ = world.despawn(e);
            }
        }
    }
}

#[cfg(feature = "server")]
impl<'a> evoke::server::DescriptorPack<'a> for TileMapDescriptor {
    type Pack = TileMapReplica;
}

#[cfg(feature = "server")]
impl evoke::server::Descriptor for TileMapDescriptor {
    type Query = &'static TileMap;
    type History = TileMapReplica;

    fn history(item: &TileMap) -> TileMapReplica {
        TileMapReplica::from_map(item)
    }

    fn replicate<'a>(
        item: &'a TileMap,
        history: Option<&TileMapReplica>,
        _scope: &'a scoped_arena::Scope<'_>,
    ) -> evoke::server::Replicate<TileMapReplica> {
        match history {
            Some(history) if history.equivalent(item) => evoke::server::Replicate::Unmodified,
            _ => evoke::server::Replicate::Modified(TileMapReplica::from_map(item)),
        }
    }
}
