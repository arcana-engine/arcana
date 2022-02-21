use sierra::{
    descriptors, ivec2, pipeline, AccessFlags, AspectFlags, Buffer, BufferView, BufferViewInfo,
    ComputePipeline, ComputePipelineInfo, ComputeShader, CreateImageError, DescriptorSetInfo,
    DescriptorSetWrite, DescriptorsAllocationError, Device, Encoder, Extent3d, Format, Image,
    ImageCopy, ImageInfo, ImageMemoryBarrier, ImageUsage, ImageViewDescriptor, ImageViewInfo,
    Layout, Offset3d, OutOfMemory, PipelineStageFlags, Samples::Samples1, ShaderModuleInfo,
    Subresource, UpdateDescriptorSet,
};

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct PushConstants {
    offset: ivec2,
    stride: u32,
}

unsafe impl bytemuck::Zeroable for PushConstants {}
unsafe impl bytemuck::Pod for PushConstants {}

#[descriptors]
struct Rgb2RgbaDescriptors {
    #[buffer(texel, uniform)]
    #[stages(Compute)]
    pixels: BufferView,

    #[image(storage)]
    #[stages(Compute)]
    image: Image,
}

#[pipeline]
struct Rgb2RgbaPipeline {
    #[set]
    set: Rgb2RgbaDescriptors,
}

pub(super) struct Rgb2RgbaUploader {
    layout: Rgb2RgbaPipelineLayout,
    pipeline: ComputePipeline,
}

impl Rgb2RgbaUploader {
    pub fn new(device: &Device) -> Result<Self, OutOfMemory> {
        // let module = device
        //     .create_shader_module(ShaderModuleInfo::glsl(
        //         include_bytes!("rgb2rgba.comp").to_owned(),
        //         ShaderStage::Compute,
        //     ))
        //     .unwrap();

        // let module = device
        //     .create_shader_module(ShaderModuleInfo::wgsl(
        //         include_bytes!("rgb2rgba.wgsl").to_owned(),
        //     ))
        //     .unwrap();

        let module = device
            .create_shader_module(ShaderModuleInfo::spirv(
                include_bytes!("rgb2rgba.comp.spv").to_owned(),
            ))
            .unwrap();

        let layout = Rgb2RgbaPipelineLayout::new(device)?;

        let pipeline = device.create_compute_pipeline(ComputePipelineInfo {
            shader: ComputeShader::new(module, "main"),
            layout: layout.raw().clone(),
        })?;

        Ok(Rgb2RgbaUploader { layout, pipeline })
    }

    pub fn upload_synchronized<'a>(
        &self,
        device: &Device,
        image: &'a Image,
        offset: Offset3d,
        extent: Extent3d,
        buffer: Buffer,
        row_length: u32,
        _image_height: u32,
        encoder: &mut Encoder<'a>,
    ) -> Result<(), OutOfMemory> {
        assert_eq!(extent.depth, 1, "3D images unsupported yet");
        assert_eq!(offset.z, 0);

        tracing::info!(
            "Dispatch RGB->RGBA upload. Image extent: '{extent:?}', offset '{offset:?}'. Buffer size: '{}', stride: '{row_length}'",
            buffer.info().size
        );

        let scope = encoder.scope();

        let mut set = device
            .create_descriptor_set(DescriptorSetInfo {
                layout: self.layout.set.raw().clone(),
            })
            .map_err(|err| match err {
                DescriptorsAllocationError::Fragmentation => unreachable!(),
                DescriptorsAllocationError::OutOfMemory { source } => source,
            })?;

        let buffer_view = device.create_buffer_view(BufferViewInfo {
            format: Format::R8Unorm,
            offset: 0,
            size: buffer.info().size,
            buffer: buffer,
        })?;

        let staging_image = device
            .create_image(ImageInfo {
                extent: image.info().extent,
                format: Format::RGBA8Unorm,
                levels: 1,
                layers: 1,
                samples: Samples1,
                usage: ImageUsage::STORAGE | ImageUsage::TRANSFER_SRC,
            })
            .map_err(|err| match err {
                CreateImageError::OutOfMemory { source } => source,
                _ => unreachable!(),
            })?;

        let staging_image = &*scope.to_scope(staging_image);

        let image_view = device.create_image_view(ImageViewInfo::new(staging_image.clone()))?;

        let mut writes = Vec::new_in(scope);
        writes.push(DescriptorSetWrite {
            binding: 0,
            element: 0,
            descriptors: sierra::Descriptors::UniformTexelBuffer(scope.to_scope([buffer_view])),
        });
        writes.push(DescriptorSetWrite {
            binding: 1,
            element: 0,
            descriptors: sierra::Descriptors::StorageImage(scope.to_scope([ImageViewDescriptor {
                view: image_view,
                layout: sierra::Layout::General,
            }])),
        });

        device.update_descriptor_sets(&mut [UpdateDescriptorSet {
            set: &mut set,
            writes: writes.leak(),
            copies: &[],
        }]);

        let pipeline = &*scope.to_scope(self.pipeline.clone());
        let layout = &*scope.to_scope(self.layout.raw().clone());
        let set = &*scope.to_scope(set.share());

        encoder.bind_compute_pipeline(pipeline);
        encoder.bind_compute_descriptor_sets(&layout, 0, scope.to_scope([set]), &[]);

        // encoder.push_constants(
        //     layout,
        //     sierra::ShaderStageFlags::COMPUTE,
        //     0,
        //     scope.to_scope([PushConstants {
        //         offset: ivec2::from([offset.x, offset.y]),
        //         stride: row_length,
        //     }]),
        // );

        encoder.image_barriers(
            PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::COMPUTE_SHADER,
            scope.to_scope([ImageMemoryBarrier {
                image: staging_image,
                old_layout: None,
                new_layout: Layout::General,
                old_access: AccessFlags::empty(),
                new_access: AccessFlags::SHADER_WRITE,
                family_transfer: None,
                range: Subresource {
                    aspect: AspectFlags::COLOR,
                    level: 0,
                    layer: 0,
                }
                .into(),
            }]),
        );
        encoder.dispatch(extent.width, extent.height, extent.depth);

        let subresource = Subresource {
            aspect: AspectFlags::COLOR,
            level: 0,
            layer: 0,
        };

        encoder.image_barriers(
            PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::TRANSFER,
            scope.to_scope([ImageMemoryBarrier {
                image: staging_image,
                old_layout: Some(Layout::General),
                new_layout: Layout::TransferSrcOptimal,
                old_access: AccessFlags::SHADER_WRITE,
                new_access: AccessFlags::TRANSFER_READ,
                family_transfer: None,
                range: subresource.into(),
            }]),
        );

        encoder.copy_image(
            staging_image,
            Layout::TransferSrcOptimal,
            image,
            Layout::TransferDstOptimal,
            scope.to_scope([ImageCopy {
                src_subresource: subresource.into(),
                src_offset: Offset3d::ZERO,
                dst_subresource: subresource.into(),
                dst_offset: Offset3d::ZERO,
                extent,
            }]),
        );

        Ok(())
    }
}
