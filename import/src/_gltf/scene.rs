use {
    super::{GltfAsset, GltfFormat},
    crate::{
        assets::{AssetHandle, AssetResult, Loader},
        graphics::{Graphics, Scale},
        prefab::Prefab,
        resources::Res,
        scene::{Global3, Local3},
    },
    edict::{EntityId, World},
    gltf::Node,
};

pub struct GltfScene {
    nodes: Box<[EntityId]>,
}

pub struct Gltf {
    key: Box<str>,
    format: GltfFormat,
}

impl Gltf {
    pub fn new(key: Box<str>) -> Self {
        Gltf {
            key,
            format: Default::default(),
        }
    }
}

impl Prefab for Gltf {
    type Loaded = AssetResult<GltfAsset>;
    type Fut = AssetHandle<GltfAsset>;

    /// Loads prefab components.
    fn load(&self, loader: &Loader) -> Self::Fut {
        loader.load_with(self.format, &self.key)
    }

    fn spawn(
        mut loaded: AssetResult<GltfAsset>,
        res: &mut Res,
        world: &mut World,
        graphics: &mut Graphics,
        entity: EntityId,
    ) -> eyre::Result<()> {
        if world.get::<Self>(entity).is_err() {
            tracing::warn!("Prefab loading aborted");
            return Ok(());
        }

        let asset = loaded.get_existing(graphics)?;

        let scene = match asset.gltf.default_scene() {
            Some(scene) => scene,
            None => asset.gltf.scenes().next().unwrap(),
        };

        match scene.nodes().len() {
            0 => return Err(eyre::eyre!("Gltf asset with 0 nodes loaded")),
            1 => {
                spawn_node(
                    None,
                    na::Vector3::new(1.0, 1.0, 1.0),
                    scene.nodes().next().unwrap(),
                    &asset,
                    world,
                );
                tracing::info!("Gltf asset loaded");
            }
            _ => {
                let nodes = scene
                    .nodes()
                    .map(|node| {
                        spawn_node(
                            Some(entity),
                            na::Vector3::new(1.0, 1.0, 1.0),
                            node,
                            &asset,
                            world,
                        )
                    })
                    .collect();

                world.try_insert(&entity, GltfScene { nodes }).unwrap();
                tracing::info!("Gltf asset loaded");
            }
        }

        Ok(())
    }
}

fn spawn_node(
    parent: Option<EntityId>,
    parent_scale: na::Vector3<f32>,
    node: Node<'_>,
    asset: &GltfAsset,
    world: &mut World,
) -> EntityId {
    let (iso, scale) = node_transform(&node, parent_scale);

    let renderables = node
        .mesh()
        .and_then(|m| asset.meshes.get(m.index()))
        .and_then(|m| m.renderables.as_deref())
        .unwrap_or(&[]);

    let entity = match renderables {
        [] => match parent {
            Some(parent) => world.spawn((Local3 { iso, parent }, Global3::identity())),
            None => world.spawn((Global3::new(iso),)),
        },
        [renderable] => {
            let renderable = renderable.clone();

            match parent {
                Some(parent) => world.spawn((
                    renderable.mesh,
                    renderable.material,
                    Scale(scale),
                    Local3 { iso, parent },
                    Global3::identity(),
                )),
                None => world.spawn((
                    renderable.mesh,
                    renderable.material,
                    Scale(scale),
                    Global3::new(iso),
                )),
            }
        }
        _ => {
            let entity = match parent {
                Some(parent) => world.spawn((Local3 { iso, parent }, Global3::identity())),
                None => world.spawn((Global3::new(iso),)),
            };
            world.spawn_batch(renderables.iter().cloned().map(|r| {
                (
                    r.mesh,
                    r.material,
                    Scale(scale),
                    Global3::identity(),
                    Local3::identity(entity),
                )
            }));
            entity
        }
    };

    spawn_children(
        entity,
        parent_scale.component_mul(&scale),
        &node,
        asset,
        world,
    );
    entity
}

fn spawn_children(
    entity: EntityId,
    scale: na::Vector3<f32>,
    node: &Node<'_>,
    asset: &GltfAsset,
    world: &mut World,
) {
    for child in node.children() {
        spawn_node(Some(entity), scale, child, asset, world);
    }
}

fn node_transform(
    node: &Node,
    parent_scale: na::Vector3<f32>,
) -> (na::Isometry3<f32>, na::Vector3<f32>) {
    let (t, r, s) = node.transform().decomposed();
    let [tx, ty, tz] = t;
    let [rx, ry, rz, rw] = r;
    let [sx, sy, sz] = s;
    (
        na::Isometry3 {
            rotation: na::Unit::new_normalize(na::Quaternion::new(rw, rx, ry, rz)),
            translation: na::Translation3::new(tx, ty, tz),
        },
        na::Vector3::new(
            sx * parent_scale.x,
            sy * parent_scale.y,
            sz * parent_scale.z,
        ),
    )
}

fn node_transform_identity(node: &Node) -> bool {
    let (t, r, s) = node.transform().decomposed();

    let [x, y, z] = s;
    if (x - 1.0).abs() > std::f32::EPSILON
        || (y - 1.0).abs() > std::f32::EPSILON
        || (z - 1.0).abs() > std::f32::EPSILON
    {
        return false;
    }

    let [x, y, z] = t;
    if x.abs() > std::f32::EPSILON || y.abs() > std::f32::EPSILON || z.abs() > std::f32::EPSILON {
        return false;
    }
    let [x, y, z, w] = r;
    x.abs() <= std::f32::EPSILON
        && y.abs() <= std::f32::EPSILON
        && z.abs() <= std::f32::EPSILON
        && (w - 1.0).abs() <= std::f32::EPSILON
}
