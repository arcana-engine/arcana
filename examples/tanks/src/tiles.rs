use std::usize;

use {
    arcana::{
        assets::{
            image::ImageAsset, Asset, AssetDefaultFormat, AssetHandle, AssetResult, Error, Format,
            Loader,
        },
        graphics::{Graphics, Material, Rect, Sprite, Texture},
        hecs::{Entity, World},
        sierra::{ImageView, SamplerInfo},
        Global2, Local2, PhysicsData2, Prefab, Res,
    },
    futures::future::BoxFuture,
    ordered_float::OrderedFloat,
    rapier2d::{
        dynamics::RigidBodyBuilder,
        geometry::{Collider, ColliderBuilder, SharedShape},
    },
    std::{collections::HashMap, sync::Arc},
};

#[derive(Clone, Debug)]
pub struct TileSet {
    tiles: Arc<[Tile]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ColliderKind {
    Wall,
}

#[derive(Clone, Debug)]
pub struct Tile {
    name: Box<str>,
    image: ImageView,
    collider: Option<ColliderKind>,
}

impl Asset for TileSet {
    type Error = Error;
    type Decoded = TileSetDecoded;
    type Builder = Graphics;

    fn build(decoded: TileSetDecoded, graphics: &mut Graphics) -> Result<Self, Error> {
        let mut tiles = Vec::new();

        for mut tile in decoded.tiles {
            let image = tile.image.get_existing(graphics)?.image.clone();

            tiles.push(Tile {
                image,
                name: tile.name,
                collider: tile.collider,
            });
        }

        Ok(TileSet {
            tiles: tiles.into(),
        })
    }
}

pub struct TileDecoded {
    name: Box<str>,
    image: AssetResult<ImageAsset>,
    collider: Option<ColliderKind>,
}
pub struct TileSetDecoded {
    tiles: Vec<TileDecoded>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TileSetFormat;

impl AssetDefaultFormat for TileSet {
    type DefaultFormat = TileSetFormat;
}

impl Format<TileSet> for TileSetFormat {
    type Error = serde_json::Error;
    type Fut = BoxFuture<'static, Result<TileSetDecoded, serde_json::Error>>;

    fn decode(self, bytes: Box<[u8]>, key: &str, loader: Loader) -> Self::Fut {
        #[derive(serde::Deserialize)]
        pub struct TileSetInfo<'a> {
            #[serde(borrow = "'a")]
            tiles: Vec<TileInfo<'a>>,
        }

        #[derive(serde::Deserialize)]
        pub struct TileInfo<'a> {
            name: Box<str>,
            url: &'a str,
            #[serde(default)]
            collider: Option<ColliderKind>,
        }

        Box::pin(async move {
            match serde_json::from_slice::<TileSetInfo>(&*bytes) {
                Ok(info) => {
                    let mut tiles = Vec::new();
                    for tile in info.tiles {
                        let image = loader.load(tile.url).await;
                        tiles.push(TileDecoded {
                            image,
                            name: tile.name,
                            collider: tile.collider,
                        })
                    }
                    Ok(TileSetDecoded { tiles })
                }
                Err(err) => Err(err),
            }
        })
    }
}

#[derive(Debug)]
pub struct TileMap {
    tileset: Box<str>,
    cell_size: f32,
    width: usize,
    cells: Vec<usize>,
}

impl TileMap {
    pub fn new(tileset: Box<str>, cell_size: f32, width: usize, cells: Vec<usize>) -> Self {
        assert_eq!(
            cells.len() % width,
            0,
            "Number of `cells` must be multiple of `width`"
        );

        TileMap {
            tileset,
            cell_size,
            width,
            cells,
        }
    }
}

impl Prefab for TileMap {
    type Loaded = AssetResult<TileSet>;
    type Fut = AssetHandle<TileSet>;

    fn load(&self, loader: &Loader) -> Self::Fut {
        loader.load(&self.tileset)
    }

    fn spawn(
        mut tileset: AssetResult<TileSet>,
        res: &mut Res,
        world: &mut World,
        graphics: &mut Graphics,
        entity: Entity,
    ) -> eyre::Result<()> {
        let tileset = tileset.get_existing(graphics)?;
        let tilemap = world.query_one_mut::<&Self>(entity)?;

        let sampler = graphics.create_sampler(SamplerInfo::default())?;

        let cell_size = tilemap.cell_size;
        let cells = tilemap.cells.clone();

        struct TileMapShapes(HashMap<(ColliderKind, OrderedFloat<f32>), SharedShape>);

        let hc = cell_size * 0.5;

        for (j, row) in cells.chunks(tilemap.width).enumerate() {
            for (i, &cell) in row.iter().enumerate() {
                let tile = match tileset.tiles.get(cell) {
                    None => {
                        tracing::error!("Missing tile in the tileset");
                        None
                    }
                    Some(tile) => Some(tile),
                };

                let albedo_coverage = tile.map(|tile| Texture {
                    image: tile.image.clone(),
                    sampler: sampler.clone(),
                });

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
                    Some(ColliderKind::Wall) => {
                        let shapes = res.with(|| TileMapShapes(HashMap::new()));

                        let shape = shapes
                            .0
                            .entry((ColliderKind::Wall, OrderedFloat(cell_size)))
                            .or_insert_with(|| SharedShape::cuboid(hc, hc))
                            .clone();

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
