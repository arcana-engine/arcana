use std::sync::Arc;

use skelly::Skelly;

use crate::graphics::{Material, Mesh};

#[derive(Clone, Debug, goods::Asset)]
pub struct Object {
    #[container]
    pub primitives: Arc<[Primitive]>,
    pub colliders: Arc<[Collider]>,
    pub skin: Option<Skin>,
    #[container]
    pub materials: Arc<[Material]>,
}

#[derive(Clone, Debug, goods::AssetField)]
pub struct Primitive {
    #[external]
    pub mesh: Mesh,
    pub material: Option<usize>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub enum Collider {
    AABB {
        extent: na::Vector3<f32>,
    },
    Convex {
        points: Vec<na::Point3<f32>>,
    },
    TriMesh {
        vertices: Vec<na::Point3<f32>>,
        indices: Vec<[u32; 3]>,
    },
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Skin {
    pub inverse_binding_matrices: Option<Vec<na::Matrix4<f32>>>,
    pub skelly: Skelly<f32, String>,
}
