use std::collections::HashMap;

use gltf::Gltf;

use super::Error;

#[derive(Clone, Copy)]
pub enum ColliderKind {
    AABB,
    Convex,
    TriMesh,
}

#[derive(serde::Serialize)]
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

pub fn load_collider(
    prim: gltf::Primitive,
    kind: ColliderKind,
    gltf: &Gltf,
    sources: &HashMap<usize, Box<[u8]>>,
) -> eyre::Result<Collider> {
    let reader = prim.reader(|buffer| match buffer.source() {
        gltf::buffer::Source::Bin => gltf.blob.as_deref(),
        gltf::buffer::Source::Uri(_) => sources.get(&buffer.index()).map(|b| &**b),
    });

    let mut positions = reader.read_positions().ok_or(Error::MissingSource)?;

    match kind {
        ColliderKind::AABB => match positions.next() {
            Some([x, y, z]) => {
                let mut mx = x.abs();
                let mut my = y.abs();
                let mut mz = z.abs();

                for [x, y, z] in positions {
                    mx = mx.max(x.abs());
                    my = my.max(y.abs());
                    mz = mz.max(z.abs());
                }

                Ok(Collider::AABB {
                    extent: na::Vector3::new(mx, my, mz),
                })
            }
            None => Err(Error::InvalidConvexShape.into()),
        },

        ColliderKind::Convex => {
            let points: Vec<_> = positions
                .map(|[x, y, z]| na::Point3::new(x, y, z))
                .collect();

            Ok(Collider::Convex { points })
        }

        ColliderKind::TriMesh => {
            let vertices: Vec<_> = positions
                .map(|[x, y, z]| na::Point3::new(x, y, z))
                .collect();

            let indices: Vec<_> = match reader.read_indices() {
                Some(indices) => triplets(indices.into_u32()),
                None => triplets(0u32..vertices.len() as u32),
            };

            Ok(Collider::TriMesh { vertices, indices })
        }
    }
}

fn triplets(mut iter: impl ExactSizeIterator<Item = u32>) -> Vec<[u32; 3]> {
    let mut result = Vec::with_capacity(iter.len() / 3);
    loop {
        match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), Some(c)) => result.push([a, b, c]),
            _ => return result,
        }
    }
}
