use {
    crate::{
        graphics::{Graphics, Material, Rect, SamplerInfo, Sprite, Texture},
        physics2::PhysicsData2,
        resources::Res,
        scene::{Global2, Local2},
    },
    goods::{Asset, AssetField, AssetResult},
    hecs::{Entity, World},
    na,
    ordered_float::OrderedFloat,
    rapier2d::{
        dynamics::RigidBodyBuilder,
        geometry::{ColliderBuilder, SharedShape},
    },
    std::{collections::HashMap, sync::Arc},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
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

#[derive(Clone, Debug, AssetField)]
pub struct Tile {
    name: Box<str>,
    #[serde(default)]
    collider: Option<ColliderKind>,
    #[container]
    texture: Texture,
}

#[derive(Clone, Debug, Asset)]
pub struct TileSet {
    #[container]
    tiles: Arc<[Tile]>,
}

#[derive(Clone, Debug, Asset)]
pub struct TileMap {
    #[external]
    set: TileSet,
    cell_size: f32,
    width: usize,
    cells: Arc<[usize]>,
}

impl TileMap {
    fn spawn(
        mut result: AssetResult<TileMap>,
        res: &mut Res,
        world: &mut World,
        graphics: &mut Graphics,
        entity: Entity,
    ) -> eyre::Result<()> {
        let tilemap = result.get(graphics)?;

        let sampler = graphics.create_sampler(SamplerInfo::default())?;

        let cell_size = tilemap.cell_size;
        let cells = tilemap.cells.clone();

        let hc = cell_size * 0.5;

        for (j, row) in cells.chunks(tilemap.width).enumerate() {
            for (i, &cell) in row.iter().enumerate() {
                let tile = match tilemap.set.tiles.get(cell) {
                    None => {
                        tracing::error!("Missing tile in the tileset");
                        None
                    }
                    Some(tile) => Some(tile),
                };

                let albedo_coverage = tile.map(|tile| tile.texture.clone());

                let local_iso =
                    na::Translation2::new(i as f32 * cell_size, j as f32 * cell_size).into();

                match tile.and_then(|tile| tile.collider) {
                    None => {
                        world.spawn((
                            Local2::new(entity, local_iso),
                            Global2::identity(),
                            Sprite {
                                pos: Rect {
                                    left: -hc,
                                    right: hc,
                                    top: -hc,
                                    bottom: hc,
                                },
                                uv: Rect {
                                    left: 0.0,
                                    right: 1.0,
                                    top: 0.0,
                                    bottom: 1.0,
                                },
                                layer: 10,
                            },
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

                        physics.colliders.insert(
                            ColliderBuilder::new(shape).build(),
                            body,
                            &mut physics.bodies,
                        );

                        world.spawn((
                            Local2::new(entity, local_iso),
                            Global2::identity(),
                            Sprite {
                                pos: Rect {
                                    left: -hc,
                                    right: hc,
                                    top: -hc,
                                    bottom: hc,
                                },
                                uv: Rect {
                                    left: 0.0,
                                    right: 1.0,
                                    top: 0.0,
                                    bottom: 1.0,
                                },
                                layer: 10,
                            },
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

        world.insert_one(entity, Global2::identity())?;
        Ok(())
    }
}
