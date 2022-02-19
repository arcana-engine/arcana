use std::{collections::HashMap, f32::EPSILON, mem::size_of};

use byteorder::{ByteOrder, LittleEndian};
use gltf::{accessor::Dimensions, Gltf};
use skelly::Skelly;

use crate::assets::model::Skin;

use super::{read_accessor, Error};

pub(super) fn load_skin(
    skin: gltf::Skin,
    gltf: &Gltf,
    buffers: &HashMap<usize, Box<[u8]>>,
) -> Result<Skin, Error> {
    let inverse_binding_matrices = match skin.inverse_bind_matrices() {
        Some(accessor) => {
            if accessor.dimensions() != Dimensions::Mat4 {
                return Err(Error::UnexpectedDimensions {
                    unexpected: accessor.dimensions(),
                    expected: &[Dimensions::Mat4],
                });
            }

            assert_eq!(
                accessor.size(),
                size_of::<na::Matrix4<f32>>(),
                "Accessor to inverse binding matrices has invalid size"
            );

            let (bytes, stride) = read_accessor(accessor.clone(), gltf, buffers)?;

            let mut inverse_binding_matrices = Vec::new();

            if cfg!(target_endian = "little") && stride == size_of::<na::Matrix4<f32>>() {
                debug_assert_eq!(
                    accessor.count() * size_of::<na::Matrix4<f32>>(),
                    bytes.len()
                );
                inverse_binding_matrices.reserve(accessor.count());
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        inverse_binding_matrices.as_mut_ptr() as *mut u8,
                        bytes.len(),
                    );
                    inverse_binding_matrices.set_len(accessor.count());
                }
            } else {
                for bytes in bytes.chunks(stride) {
                    let mut a = [0.0; 16];
                    LittleEndian::read_f32_into(bytes, &mut a);

                    let m = [
                        [a[0], a[1], a[2], a[3]],
                        [a[4], a[5], a[6], a[7]],
                        [a[8], a[9], a[10], a[11]],
                        [a[12], a[13], a[14], a[15]],
                    ];

                    inverse_binding_matrices.push(na::Matrix4::from(m));
                }
            }
            Some(inverse_binding_matrices)
        }
        None => None,
    };

    let mut skelly = Skelly::new();

    for (idx, joint) in skin.joints().enumerate() {
        let parent = skin
            .joints()
            .take(idx)
            .position(|j| j.children().any(|c| c.index() == joint.index()));

        let (pos, rot, scale) = joint.transform().decomposed();
        assert!(
            scale[0] > 1.0 - EPSILON && scale[0] < 1.0 + EPSILON,
            "Joint nodes scale must be [1, 1, 1]"
        );
        assert!(
            scale[1] > 1.0 - EPSILON && scale[1] < 1.0 + EPSILON,
            "Joint nodes scale must be [1, 1, 1]"
        );
        assert!(
            scale[2] > 1.0 - EPSILON && scale[2] < 1.0 + EPSILON,
            "Joint nodes scale must be [1, 1, 1]"
        );

        let rotation = na::Unit::new_normalize(na::Quaternion::new(rot[0], rot[1], rot[2], rot[3]));
        let name = joint.name().unwrap_or("unnamed").to_owned();

        let bone = match parent {
            None => skelly.add_root_with(na::Point3::new(pos[0], pos[1], pos[2]), name),
            Some(parent) => {
                skelly.attach_with(na::Vector3::new(pos[0], pos[1], pos[2]), parent, name)
            }
        };
        assert_eq!(bone, idx);
        skelly.set_orientation(idx, rotation);
    }

    for (idx, joint) in skin.joints().enumerate() {
        assert_eq!(
            joint.children().len(),
            skelly.iter_children(idx).count(),
            "All children of a joint must be joints of the same skeleton"
        );
    }

    Ok(Skin {
        inverse_binding_matrices,
        skelly,
    })
}
