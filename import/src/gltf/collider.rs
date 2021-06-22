use {
    super::{ColliderKind, GltfBuildContext, Collider, GltfLoadingError},
    parry3d::shape::SharedShape,
};

impl GltfBuildContext<'_> {
    pub fn create_collider(
        &mut self,
        prim: gltf::Primitive,
        kind: ColliderKind,
    ) -> Result<Collider, GltfLoadingError> {
        let reader = prim.reader(|buffer| match buffer.source() {
            gltf::buffer::Source::Bin => self.decoded.gltf.blob.as_deref(),
            gltf::buffer::Source::Uri(uri) => self.decoded.sources.get(uri).map(|b| &**b),
        });

        let mut positions = reader
            .read_positions()
            .ok_or(GltfLoadingError::MissingSource)?;

        match kind {
            ColliderKind::AABB => match positions.next() {
                Some([x, y, z]) => {
                    let mut xx = [x, x];
                    let mut yy = [y, y];
                    let mut zz = [z, z];

                    for [x, y, z] in positions {
                        xx[0] = xx[0].min(x);
                        xx[1] = xx[1].max(x);

                        yy[0] = yy[0].min(y);
                        yy[1] = yy[1].max(y);

                        zz[0] = zz[0].min(z);
                        zz[1] = zz[1].max(z);
                    }

                    let shape = SharedShape::convex_mesh(
                        vec![
                            na::Point3::new(xx[0], yy[0], zz[0]),
                            na::Point3::new(xx[0], yy[0], zz[1]),
                            na::Point3::new(xx[0], yy[1], zz[0]),
                            na::Point3::new(xx[0], yy[1], zz[1]),
                            na::Point3::new(xx[1], yy[0], zz[0]),
                            na::Point3::new(xx[1], yy[0], zz[1]),
                            na::Point3::new(xx[1], yy[1], zz[0]),
                            na::Point3::new(xx[1], yy[1], zz[1]),
                        ],
                        &[
                            [0, 1, 2],
                            [2, 1, 3],
                            [0, 4, 1],
                            [1, 4, 5],
                            [2, 3, 6],
                            [6, 3, 7],
                            [4, 6, 5],
                            [5, 6, 7],
                        ],
                    )
                    .unwrap();

                    Ok(Collider { shape })
                }
                None => Err(GltfLoadingError::InvalidConvexShape),
            },

            ColliderKind::Convex => {
                let positions: Vec<_> = positions
                    .map(|[x, y, z]| na::Point3::new(x, y, z))
                    .collect();

                let indices: Vec<_> = match reader.read_indices() {
                    Some(indices) => triplets(indices.into_u32()),
                    None => triplets(0u32..positions.len() as u32),
                };

                let shape = SharedShape::convex_mesh(positions, &indices)
                    .ok_or(GltfLoadingError::InvalidConvexShape)?;

                Ok(Collider { shape })
            }

            ColliderKind::TriMesh => {
                let positions: Vec<_> = positions
                    .map(|[x, y, z]| na::Point3::new(x, y, z))
                    .collect();

                let indices: Vec<_> = match reader.read_indices() {
                    Some(indices) => triplets(indices.into_u32()),
                    None => triplets(0u32..positions.len() as u32),
                };

                let shape = SharedShape::trimesh(positions, indices);
                Ok(Collider { shape })
            }
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
