use std::collections::HashMap;

use gltf::Gltf;

use crate::assets::model::Collider;

use super::Error;

#[derive(Clone, Copy)]
pub enum ColliderKind {
    AABB,
    Convex,
    TriMesh,
}

pub(super) fn load_collider(
    prim: gltf::Primitive,
    kind: ColliderKind,
    gltf: &Gltf,
    buffers: &HashMap<usize, Box<[u8]>>,
) -> Result<Collider, Error> {
    let reader = prim.reader(|buffer| match buffer.source() {
        gltf::buffer::Source::Bin => gltf.blob.as_deref(),
        gltf::buffer::Source::Uri(_) => buffers.get(&buffer.index()).map(|b| &**b),
    });

    let mut positions = reader.read_positions().unwrap();

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
                    extent: na::Vector3::from([mx, my, mz]),
                })
            }
            None => Err(Error::InvalidConvexShape.into()),
        },

        ColliderKind::Convex => {
            let points: Vec<_> = positions
                .map(|[x, y, z]| na::Point3::from([x, y, z]))
                .collect();

            Ok(Collider::Convex { points })
        }

        ColliderKind::TriMesh => {
            let vertices: Vec<_> = positions
                .map(|[x, y, z]| na::Point3::from([x, y, z]))
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
