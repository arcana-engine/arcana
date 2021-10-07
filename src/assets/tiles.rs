use std::{collections::HashMap, fmt, sync::Arc};

#[cfg(any(feature = "client", feature = "server"))]
use alkahest::Bytes;

use goods::{Asset, Loader};
use hecs::{Entity, World};
use na;
use ordered_float::OrderedFloat;
use rapier2d::{
    dynamics::RigidBodyBuilder,
    geometry::{ColliderBuilder, SharedShape},
};
use tracing::{instrument, Instrument};
use uuid::Uuid;

use crate::{prefab::PrefabComponent, task::with_async_task_context, Spawner};

use {
    super::WithUuid,
    crate::{
        physics2::PhysicsData2,
        resources::Res,
        scene::{Global2, Local2},
    },
};

#[cfg(feature = "visible")]
use {
    crate::graphics::{Graphics, Material, Rect, Sprite, Texture},
    goods::AssetField,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[cfg_attr(feature = "server", derive(serde::Serialize))]
#[serde(rename_all = "snake_case")]
enum ColliderKind {
    Wall,
}

impl ColliderKind {
    pub fn shared_shape(&self, size: f32, res: &mut Res) -> SharedShape {
        struct TileMapShapes(HashMap<(ColliderKind, OrderedFloat<f32>), SharedShape>);
        let shapes = res.with(|| TileMapShapes(HashMap::new()));

        match shapes.0.get(&(*self, OrderedFloat(size))) {
            Some(shape) => shape.clone(),
            None => {
                let shape = SharedShape::cuboid(size * 0.5, size * 0.5);
                shapes.0.insert((*self, OrderedFloat(size)), shape.clone());
                shape
            }
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "visible", derive(AssetField))]
#[cfg_attr(not(feature = "visible"), derive(serde::Deserialize))]
#[cfg_attr(feature = "server", derive(serde::Serialize))]
pub struct Tile {
    #[serde(default)]
    collider: Option<ColliderKind>,

    #[cfg(feature = "visible")]
    #[serde(default, skip_serializing)]
    #[container]
    texture: Option<Texture>,
}

#[derive(Clone, Debug, Asset)]
#[cfg_attr(feature = "server", derive(serde::Serialize))]
pub struct TileSet {
    #[cfg_attr(feature = "visible", container)]
    tiles: Arc<[Tile]>,
}

#[derive(Clone, Debug, Asset)]
pub struct TileMap {
    #[external]
    set: WithUuid<TileSet>,
    cell_size: f32,
    width: usize,
    cells: Arc<[usize]>,
}

#[cfg_attr(feature = "client", derive(serde::Deserialize))]
#[cfg_attr(feature = "server", derive(serde::Serialize))]
pub struct TileMapComponent {
    pub set: Uuid,
    pub cell_size: f32,
    pub width: usize,
    pub cells: Arc<[usize]>,
}

impl fmt::Debug for TileMapComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TileMapComponent")
            .field("set", &self.set)
            .field("cell_size", &self.cell_size)
            .field("width", &self.width)
            .field("cells.len()", &self.cells.len())
            .finish()
    }
}

#[cfg(feature = "client")]
impl PrefabComponent for TileMapComponent {
    #[instrument(skip(world, res, spawner))]
    fn pre_insert(
        &mut self,
        entity: Entity,
        world: &mut World,
        res: &mut Res,
        spawner: &mut crate::Spawner,
    ) {
        let set = self.set;
        let cell_size = self.cell_size;
        let width = self.width;
        let cells = self.cells.clone();

        spawner.spawn(
            async move {
                let mut set = with_async_task_context(|cx| cx.loader.load::<TileSet>(&set)).await;

                tracing::debug!("TileSet for TileMapComponent loaded");

                with_async_task_context(|cx| {
                    let set = set.get(cx.graphics)?;

                    let origin = match cx.world.query_one_mut::<&Global2>(entity) {
                        Err(_) => return Ok(()),
                        Ok(global) => global.iso,
                    };

                    spawn_tiles(
                        Some(entity),
                        &origin,
                        set,
                        &cells,
                        cell_size,
                        width,
                        cx.world,
                        cx.res,
                    )?;

                    Ok(())
                })
            }
            .in_current_span(),
        );
    }
}

#[cfg_attr(feature = "client", derive(serde::Deserialize))]
#[cfg_attr(feature = "server", derive(serde::Serialize))]
pub struct TileComponent {
    pub set: Uuid,
    pub cell: usize,
}

impl TileMap {
    fn spawn(
        &self,
        origin: &na::Isometry2<f32>,
        res: &mut Res,
        world: &mut World,
        spawner: &mut Spawner,
    ) -> eyre::Result<Entity> {
        let cell_size = self.cell_size;
        let cells = self.cells.clone();

        let entity = world.spawn((
            TileMapComponent {
                set: WithUuid::uuid(&self.set),
                cell_size: self.cell_size,
                width: self.width,
                cells: self.cells.clone(),
            },
            Global2::new(*origin),
        ));

        spawn_tiles(
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

    fn spawn_individual_tiles(
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
        spawner: &mut Spawner,
        #[cfg(feature = "visible")] graphics: &mut Graphics,
    ) -> eyre::Result<Entity> {
        let mut map = loader.load::<Self>(uuid).await;
        #[cfg(feature = "visible")]
        let map = map.get(graphics)?;
        #[cfg(not(feature = "visible"))]
        let map = map.get(&mut ())?;

        map.spawn(origin, res, world, spawner)
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

        map.spawn_individual_tiles(origin, world, res)
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
) -> eyre::Result<()> {
    let hc = cell_size * 0.5;
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
                }
                Some(collider) => {
                    let shape = collider.shared_shape(cell_size, res);

                    let physics = res.with(PhysicsData2::new);
                    let body = physics
                        .bodies
                        .insert(RigidBodyBuilder::new_static().build());

                    physics.colliders.insert_with_parent(
                        ColliderBuilder::new(shape).build(),
                        body,
                        &mut physics.bodies,
                    );

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
                        body,
                    ));

                    if let Some(map) = map {
                        world
                            .insert_one(e, Local2::new(map, local_tr.into()))
                            .unwrap();
                    }
                }
            }
        }
    }
    Ok(())
}
