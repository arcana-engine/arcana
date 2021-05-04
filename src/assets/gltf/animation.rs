use {
    super::{
        read_accessor, GltfAnimation, GltfBuildContext, GltfChannel, GltfLoadingError,
        GltfSamplerOutput,
    },
    byteorder::{ByteOrder as _, LittleEndian},
    gltf::accessor::{DataType, Dimensions},
    std::mem::size_of,
};

impl GltfBuildContext<'_> {
    pub fn create_animation(
        &mut self,
        animation: gltf::Animation,
    ) -> Result<GltfAnimation, GltfLoadingError> {
        let channels = animation
            .channels()
            .map(|channel| {
                let sampler = channel.sampler();
                let input = sampler.input();
                if input.data_type() != DataType::F32 {
                    return Err(GltfLoadingError::UnexpectedDataType {
                        expected: &[DataType::F32],
                        unexpected: input.data_type(),
                    });
                }

                if input.dimensions() != Dimensions::Scalar {
                    return Err(GltfLoadingError::UnexpectedDimensions {
                        expected: &[Dimensions::Scalar],
                        unexpected: input.dimensions(),
                    });
                }

                let (bytes, stride) = read_accessor(input.clone(), &self.decoded)?;
                let mut input_vec = Vec::with_capacity(input.count());

                if cfg!(target_endian = "little") && stride == size_of::<f32>() {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            bytes.as_ptr(),
                            input_vec.as_mut_ptr() as *mut u8,
                            size_of::<f32>() * input.count(),
                        );
                        input_vec.set_len(input.count());
                    }
                } else {
                    input_vec.extend(
                        bytes
                            .chunks(stride)
                            .map(|bytes| LittleEndian::read_f32(&bytes[..size_of::<f32>()])),
                    )
                }

                let output = sampler.output();

                if output.data_type() != DataType::F32 {
                    return Err(GltfLoadingError::UnexpectedDataType {
                        expected: &[DataType::F32],
                        unexpected: output.data_type(),
                    });
                }

                let output =
                    match output.dimensions() {
                        Dimensions::Scalar => {
                            let (bytes, stride) = read_accessor(output.clone(), &self.decoded)?;
                            let mut output_vec = Vec::with_capacity(output.count());

                            if cfg!(target_endian = "little") && stride == size_of::<f32>() {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(
                                        bytes.as_ptr(),
                                        output_vec.as_mut_ptr() as *mut u8,
                                        size_of::<f32>() * output.count(),
                                    );
                                    output_vec.set_len(output.count());
                                }
                            } else {
                                output_vec.extend(bytes.chunks(stride).map(|bytes| {
                                    LittleEndian::read_f32(&bytes[..size_of::<f32>()])
                                }))
                            }

                            GltfSamplerOutput::Scalar(output_vec.into())
                        }
                        Dimensions::Vec2 => {
                            let (bytes, stride) = read_accessor(output.clone(), &self.decoded)?;
                            let mut output_vec = Vec::with_capacity(output.count());

                            if cfg!(target_endian = "little") && stride == size_of::<[f32; 2]>() {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(
                                        bytes.as_ptr(),
                                        output_vec.as_mut_ptr() as *mut u8,
                                        size_of::<f32>() * output.count(),
                                    );
                                    output_vec.set_len(output.count());
                                }
                            } else {
                                output_vec.extend(bytes.chunks(stride).map(|bytes| {
                                    let mut a = [0.0; 2];
                                    LittleEndian::read_f32_into(
                                        &bytes[..size_of::<[f32; 2]>()],
                                        &mut a,
                                    );
                                    a
                                }))
                            }

                            GltfSamplerOutput::Vec2(output_vec.into())
                        }
                        Dimensions::Vec3 => {
                            let (bytes, stride) = read_accessor(output.clone(), &self.decoded)?;
                            let mut output_vec = Vec::with_capacity(output.count());

                            if cfg!(target_endian = "little") && stride == size_of::<[f32; 3]>() {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(
                                        bytes.as_ptr(),
                                        output_vec.as_mut_ptr() as *mut u8,
                                        size_of::<f32>() * output.count(),
                                    );
                                    output_vec.set_len(output.count());
                                }
                            } else {
                                output_vec.extend(bytes.chunks(stride).map(|bytes| {
                                    let mut a = [0.0; 3];
                                    LittleEndian::read_f32_into(
                                        &bytes[..size_of::<[f32; 3]>()],
                                        &mut a,
                                    );
                                    a
                                }))
                            }

                            GltfSamplerOutput::Vec3(output_vec.into())
                        }
                        Dimensions::Vec4 => {
                            let (bytes, stride) = read_accessor(output.clone(), &self.decoded)?;
                            let mut output_vec = Vec::with_capacity(output.count());

                            if cfg!(target_endian = "little") && stride == size_of::<[f32; 4]>() {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(
                                        bytes.as_ptr(),
                                        output_vec.as_mut_ptr() as *mut u8,
                                        size_of::<f32>() * output.count(),
                                    );
                                    output_vec.set_len(output.count());
                                }
                            } else {
                                output_vec.extend(bytes.chunks(stride).map(|bytes| {
                                    let mut a = [0.0; 4];
                                    LittleEndian::read_f32_into(
                                        &bytes[..size_of::<[f32; 4]>()],
                                        &mut a,
                                    );
                                    a
                                }))
                            }

                            GltfSamplerOutput::Vec4(output_vec.into())
                        }
                        dim => {
                            return Err(GltfLoadingError::UnexpectedDimensions {
                                unexpected: dim,
                                expected: &[
                                    Dimensions::Scalar,
                                    Dimensions::Vec2,
                                    Dimensions::Vec3,
                                    Dimensions::Vec4,
                                ],
                            })
                        }
                    };

                Ok(GltfChannel {
                    input: input_vec.into(),
                    output,
                    interpolation: sampler.interpolation(),

                    node: channel.target().node().index(),
                    property: channel.target().property(),
                })
            })
            .collect::<Result<_, _>>()?;

        Ok(GltfAnimation { channels })
    }
}
