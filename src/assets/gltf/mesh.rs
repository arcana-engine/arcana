use {
    super::{ColliderKind, GltfBuildContext, GltfLoadingError, GltfMesh},
    std::sync::Arc,
};

impl GltfBuildContext<'_> {
    pub fn create_mesh(&mut self, mesh: gltf::Mesh) -> Result<GltfMesh, GltfLoadingError> {
        let purpose = mesh_purpose(&mesh);
        let mut gltf_mesh = GltfMesh {
            renderables: None,
            colliders: None,
            skin: None,
        };

        if purpose.render {
            gltf_mesh.renderables = Some(
                mesh.primitives()
                    .map(|prim| self.create_primitive(prim))
                    .collect::<Result<Arc<[_]>, _>>()?,
            );
        }
        if let Some(collider) = purpose.collider {
            gltf_mesh.colliders = Some(
                mesh.primitives()
                    .map(|prim| self.create_collider(prim, collider))
                    .collect::<Result<Arc<[_]>, _>>()?,
            );
        }
        Ok(gltf_mesh)
    }
}

struct MeshPurpose {
    render: bool,
    collider: Option<ColliderKind>,
}

fn mesh_purpose(mesh: &gltf::Mesh) -> MeshPurpose {
    match mesh.name() {
        Some("draw") => MeshPurpose {
            render: true,
            collider: None,
        },
        Some("aabb") => MeshPurpose {
            render: false,
            collider: Some(ColliderKind::AABB),
        },
        Some("convex") => MeshPurpose {
            render: false,
            collider: Some(ColliderKind::Convex),
        },
        Some("trimesh") => MeshPurpose {
            render: false,
            collider: Some(ColliderKind::TriMesh),
        },
        Some("draw+aabb") => MeshPurpose {
            render: true,
            collider: Some(ColliderKind::AABB),
        },
        Some("draw+convex") => MeshPurpose {
            render: true,
            collider: Some(ColliderKind::Convex),
        },
        Some("draw+trimesh") => MeshPurpose {
            render: true,
            collider: Some(ColliderKind::TriMesh),
        },
        _ => MeshPurpose {
            render: true,
            collider: None,
        },
    }
}
