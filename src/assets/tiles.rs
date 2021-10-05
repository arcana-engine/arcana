use {
    super::WithUuid,
    crate::{
        physics2::PhysicsData2,
        resources::Res,
        scene::{Global2, Local2},
    },
    goods::{Asset, Loader},
    hecs::{Entity, World},
    na,
    ordered_float::OrderedFloat,
    rapier2d::{
        dynamics::RigidBodyBuilder,
        geometry::{ColliderBuilder, SharedShape},
    },
    std::{collections::HashMap, sync::Arc},
    uuid::Uuid,
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

#[cfg_attr(feature = "visible", derive(serde::Deserialize))]
#[cfg_attr(feature = "server", derive(serde::Serialize))]
pub struct TileMapComponent {
    pub set: Uuid,
    pub cell_size: f32,
    pub width: usize,
    pub cells: Arc<[usize]>,
}

#[cfg_attr(feature = "visible", derive(serde::Deserialize))]
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
    ) -> eyre::Result<Entity> {
        let cell_size = self.cell_size;
        let cells = self.cells.clone();

        let hc = cell_size * 0.5;

        let map_entity = world.spawn((
            TileMapComponent {
                set: WithUuid::uuid(&self.set),
                cell_size: self.cell_size,
                width: self.width,
                cells: self.cells.clone(),
            },
            Global2::new(*origin),
        ));

        for (j, row) in cells.chunks(self.width).enumerate() {
            for (i, &cell) in row.iter().enumerate() {
                let tile = match self.set.tiles.get(cell) {
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
                        world.spawn((
                            Local2::new(map_entity, local_tr.into()),
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

                        world.spawn((
                            Local2::new(map_entity, local_tr.into()),
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
                    }
                }
            }
        }

        Ok(map_entity)
    }

    fn spawn_individual_tiles(
        &self,
        origin: &na::Isometry2<f32>,
        res: &mut Res,
        world: &mut World,
    ) -> eyre::Result<()> {
        let cell_size = self.cell_size;
        let cells = self.cells.clone();

        let hc = cell_size * 0.5;

        for (j, row) in cells.chunks(self.width).enumerate() {
            for (i, &cell) in row.iter().enumerate() {
                let tile = match self.set.tiles.get(cell) {
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
                        world.spawn((
                            TileComponent {
                                set: WithUuid::uuid(&self.set),
                                cell,
                            },
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

                        world.spawn((
                            TileComponent {
                                set: WithUuid::uuid(&self.set),
                                cell,
                            },
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
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn load_and_spawn(
        uuid: &Uuid,
        origin: &na::Isometry2<f32>,
        loader: &Loader,
        res: &mut Res,
        world: &mut World,
        #[cfg(feature = "visible")] graphics: &mut Graphics,
    ) -> eyre::Result<Entity> {
        let mut map = loader.load::<Self>(uuid).await;
        #[cfg(feature = "visible")]
        let map = map.get(graphics)?;
        #[cfg(not(feature = "visible"))]
        let map = map.get(&mut ())?;

        map.spawn(origin, res, world)
    }

    pub async fn load_and_spawn_individual_tiles(
        uuid: &Uuid,
        origin: &na::Isometry2<f32>,
        loader: &Loader,
        res: &mut Res,
        world: &mut World,
        #[cfg(feature = "visible")] graphics: &mut Graphics,
    ) -> eyre::Result<()> {
        let mut map = loader.load::<Self>(uuid).await;
        #[cfg(feature = "visible")]
        let map = map.get(graphics)?;
        #[cfg(not(feature = "visible"))]
        let map = map.get(&mut ())?;

        map.spawn_individual_tiles(origin, res, world)
    }
}
