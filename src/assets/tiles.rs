use std::{collections::HashMap, convert::TryFrom, fmt, sync::Arc};

use alkahest::{Schema, Seq, Unpacked};
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

#[cfg(feature = "server")]
use scoped_arena::Scope;

use crate::{task::with_async_task_context, Spawner};

#[cfg(feature = "client")]
use crate::net::client;

#[cfg(feature = "server")]
use crate::net::server;

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
pub struct Tile {
    #[serde(default)]
    collider: Option<ColliderKind>,

    #[cfg(feature = "visible")]
    #[serde(default, skip_serializing)]
    #[container]
    texture: Option<Texture>,
}

#[derive(Clone, Debug, Asset)]
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

#[cfg(any(feature = "client", feature = "server"))]
#[derive(Schema)]
pub struct TileMapComponentReplica {
    pub set: u128,
    pub cell_size: f32,
    pub width: u32,
    pub cells: Seq<u32>,
}

#[cfg(any(feature = "client", feature = "server"))]
fn pack_cell_index(cell: usize) -> u32 {
    u32::try_from(cell).expect("Too large tile map")
}

#[cfg(any(feature = "client", feature = "server"))]
fn unpack_cell_index(cell: u32) -> usize {
    usize::try_from(cell).expect("Too large tile map")
}

#[cfg(feature = "client")]
impl client::ReplicaSetElem for TileMapComponent {
    type Component = Self;
    type Replica = TileMapComponentReplica;

    #[inline(always)]
    fn build(unpacked: Unpacked<'_, TileMapComponentReplica>) -> Self {
        TileMapComponent {
            set: Uuid::from_u128(unpacked.set),
            cell_size: unpacked.cell_size,
            width: unpack_cell_index(unpacked.width),
            cells: unpacked.cells.map(unpack_cell_index).collect(),
        }
    }

    #[inline(always)]
    fn replicate(unpacked: Unpacked<'_, Self::Replica>, component: &mut Self) {
        component.set = Uuid::from_u128(unpacked.set);
        component.cell_size = unpacked.cell_size;
        component.width = unpack_cell_index(unpacked.width);
        component.cells = unpacked.cells.map(unpack_cell_index).collect();
    }

    #[instrument(skip(world, res, spawner))]
    fn pre_insert(
        component: &mut Self::Component,
        entity: Entity,
        world: &mut World,
        res: &mut Res,
        spawner: &mut Spawner,
    ) {
        let set = component.set;
        let cell_size = component.cell_size;
        let width = component.width;
        let cells = component.cells.clone();

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

#[cfg(feature = "server")]
impl<'a> server::ReplicaSetElem<'a> for TileMapComponent {
    type Component = Self;
    type Replica = TileMapComponentReplica;
    type ReplicaPack = TileMapComponentReplicaPack<u128, f32, u32, &'a [u32]>;

    fn replicate(component: &'a Self, scope: &'a Scope<'_>) -> Self::ReplicaPack {
        TileMapComponentReplicaPack {
            set: component.set.as_u128(),
            cell_size: component.cell_size,
            width: pack_cell_index(component.width),
            cells: &*scope
                .to_scope_from_iter(component.cells.iter().map(|&cell| pack_cell_index(cell))),
        }
    }
}

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
