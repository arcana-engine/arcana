use {
    super::{read_accessor, GltfBuildContext, GltfLoadingError, Skin},
    byteorder::{ByteOrder as _, LittleEndian},
    gltf::accessor::Dimensions,
    std::{collections::hash_map::Entry, mem::size_of},
};

impl GltfBuildContext<'_> {
    pub fn get_skin(&mut self, skin: gltf::Skin) -> Result<Skin, GltfLoadingError> {
        let skin_index = skin.index();
        match self.skins.entry(skin_index) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(_) => {
                let gltf_skin = self.create_skin(skin)?;
                Ok(self.skins.entry(skin_index).or_insert(gltf_skin).clone())
            }
        }
    }

    fn create_skin(&mut self, skin: gltf::Skin) -> Result<Skin, GltfLoadingError> {
        match skin.inverse_bind_matrices() {
            Some(accessor) => {
                if accessor.dimensions() != Dimensions::Mat4 {
                    return Err(GltfLoadingError::UnexpectedDimensions {
                        unexpected: accessor.dimensions(),
                        expected: &[Dimensions::Mat4],
                    });
                }

                assert_eq!(accessor.size(), size_of::<na::Matrix4<f32>>());

                let (bytes, stride) = read_accessor(accessor.clone(), &self.decoded)?;

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

                Ok(Skin {
                    inverse_binding_matrices: Some(inverse_binding_matrices.into()),
                    joints: skin.joints().map(|j| j.index()).collect(),
                })
            }
            None => Ok(Skin {
                inverse_binding_matrices: None,
                joints: skin.joints().map(|j| j.index()).collect(),
            }),
        }
    }
}
