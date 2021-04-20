use {
    super::{GltfAsset, GltfFormat, GltfRenderable},
    crate::{
        assets::{AssetHandle, AssetResult, Loader},
        graphics::{Graphics, Scale},
        prefab::Prefab,
        resources::Res,
        scene::{Global3, Local3},
    },
    gltf::Node,
    hecs::{Entity, World},
};

pub struct GltfScene {
    nodes: Box<[Entity]>,
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
        entity: Entity,
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
            1 if node_transform_identity(&scene.nodes().next().unwrap()) => {
                tracing::info!("Gltf asset with single node at origin");

                let node = scene.nodes().next().unwrap();
                let (iso, scale) = node_transform(&node, na::Vector3::new(1.0, 1.0, 1.0));

                match node.mesh().and_then(|m| asset.renderables.get(m.index())) {
                    Some(renderables) => match &**renderables {
                        [renderable] => {
                            world
                                .insert(
                                    entity,
                                    (
                                        renderable.mesh.clone(),
                                        renderable.material.clone(),
                                        Scale(scale),
                                        Global3::new(iso),
                                    ),
                                )
                                .unwrap();
                        }
                        _ => {
                            world.spawn_batch(renderables.iter().cloned().map(|r| {
                                (
                                    r.mesh,
                                    r.material,
                                    Scale(scale),
                                    Local3::identity(entity),
                                    Global3::new(iso),
                                )
                            }));
                        }
                    },
                    None => {}
                };

                spawn_children(entity, scale, &node, &asset, world);
            }
            _ => {
                tracing::info!("Gltf asset loaded");
                let nodes = scene
                    .nodes()
                    .map(|node| {
                        spawn_node(entity, na::Vector3::new(1.0, 1.0, 1.0), node, &asset, world)
                    })
                    .collect();

                world.insert(entity, (GltfScene { nodes },)).unwrap();
            }
        }

        Ok(())
    }
}

fn spawn_node(
    parent: Entity,
    parent_scale: na::Vector3<f32>,
    node: Node<'_>,
    asset: &GltfAsset,
    world: &mut World,
) -> Entity {
    let (iso, scale) = node_transform(&node, parent_scale);

    let entity = match node.mesh().and_then(|m| asset.renderables.get(m.index())) {
        Some(renderables) => match renderables.len() {
            0 => spawn_empty(parent, iso, world),
            1 => {
                let mut renderable = renderables[0].clone();
                renderable.transform = na::Matrix4::new_nonuniform_scaling(&scale);

                world.spawn((
                    renderable.mesh,
                    renderable.material,
                    Scale(scale),
                    Local3 { iso, parent },
                    Global3::identity(),
                ))
            }
            _ => {
                let entity = spawn_empty(parent, iso, world);
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
        },
        None => spawn_empty(parent, iso, world),
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
    entity: Entity,
    scale: na::Vector3<f32>,
    node: &Node<'_>,
    asset: &GltfAsset,
    world: &mut World,
) {
    for child in node.children() {
        spawn_node(entity, scale, child, asset, world);
    }
}

fn spawn_empty(parent: Entity, iso: na::Isometry3<f32>, world: &mut World) -> Entity {
    let local = Local3 { iso, parent };
    world.spawn((local, Global3::identity()))
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
