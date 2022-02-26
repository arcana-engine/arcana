use std::collections::HashMap;

use gltf::Gltf;

use crate::model::{Collider, PrimitiveInfo};

use super::{
    collider::{load_collider, ColliderKind},
    primitive::load_primitive,
    Error,
};

pub struct Mesh {
    pub primitives: Vec<PrimitiveInfo>,
    pub colliders: Vec<Collider>,
}

pub(super) fn load_mesh(
    mesh: gltf::Mesh,
    gltf: &Gltf,
    buffers: &HashMap<usize, Box<[u8]>>,
    output: &mut Vec<u8>,
) -> Result<Mesh, Error> {
    let purpose = purpose(&mesh);

    let mut output_mesh = Mesh {
        primitives: Vec::new(),
        colliders: Vec::new(),
    };

    if purpose.render {
        output_mesh.primitives = mesh
            .primitives()
            .map(|prim| load_primitive(prim, gltf, buffers, output))
            .collect::<Result<_, _>>()?;
    }
    if let Some(collider) = purpose.collider {
        output_mesh.colliders = mesh
            .primitives()
            .map(|prim| load_collider(prim, collider, gltf, buffers))
            .collect::<Result<_, _>>()?;
    }
    Ok(output_mesh)
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
                        collider: Some(ColliderKind::Aabb),
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
                        collider: Some(ColliderKind::Aabb),
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
