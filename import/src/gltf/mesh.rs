use std::{collections::HashMap, path::Path};

use gltf::Gltf;
use goods_treasury_import::Registry;

use super::{
    collider::{load_collider, Collider, ColliderKind},
    primitive::{load_primitive, Primitive},
};

pub struct Mesh {
    pub primitives: Vec<Primitive>,
    pub colliders: Vec<Collider>,
}

pub fn load_mesh(
    mesh: gltf::Mesh,
    gltf: &Gltf,
    sources: &HashMap<usize, Box<[u8]>>,
    path: &Path,
    registry: &mut dyn Registry,
) -> eyre::Result<Mesh> {
    let purpose = purpose(&mesh);

    let mut output = Mesh {
        primitives: Vec::new(),
        colliders: Vec::new(),
    };

    if purpose.render {
        output.primitives = mesh
            .primitives()
            .map(|prim| load_primitive(prim, gltf, sources, path, registry))
            .collect::<Result<_, _>>()?;
    }
    if let Some(collider) = purpose.collider {
        output.colliders = mesh
            .primitives()
            .map(|prim| load_collider(prim, collider, gltf, sources))
            .collect::<Result<_, _>>()?;
    }
    Ok(output)
}

#[derive(Clone, Copy)]
struct MeshPurpose {
    render: bool,
    collider: Option<ColliderKind>,
}

fn purpose(mesh: &gltf::Mesh) -> MeshPurpose {
    if let Some(name) = mesh.name() {
        match name.rfind('.') {
            Some(pos) => {
                let ext = name[pos + 1..].trim();

                match ext {
                    "draw" => MeshPurpose {
                        render: true,
                        collider: None,
                    },
                    "aabb" => MeshPurpose {
                        render: false,
                        collider: Some(ColliderKind::AABB),
                    },
                    "convex" => MeshPurpose {
                        render: false,
                        collider: Some(ColliderKind::Convex),
                    },
                    "trimesh" => MeshPurpose {
                        render: false,
                        collider: Some(ColliderKind::TriMesh),
                    },
                    "draw+aabb" => MeshPurpose {
                        render: true,
                        collider: Some(ColliderKind::AABB),
                    },
                    "draw+convex" => MeshPurpose {
                        render: true,
                        collider: Some(ColliderKind::Convex),
                    },
                    "draw+trimesh" => MeshPurpose {
                        render: true,
                        collider: Some(ColliderKind::TriMesh),
                    },
                    _ => MeshPurpose {
                        render: true,
                        collider: None,
                    },
                }
            }
            None => MeshPurpose {
                render: true,
                collider: None,
            },
        }
    } else {
        MeshPurpose {
            render: true,
            collider: None,
        }
    }
}
