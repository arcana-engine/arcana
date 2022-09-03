use parking_lot::Mutex;
use sierra::{
    ivec2, Access, AspectFlags, Buffer, BufferView, BufferViewInfo, ComputePipeline,
    ComputePipelineInfo, ComputeShader, Descriptors, DescriptorsAllocationError, Device, Encoder,
    Extent3, Format, Image, ImageCopy, ImageInfo, ImageMemoryBarrier, ImageUsage, Layout, Offset3,
    OutOfMemory, PipelineInput, PipelineStages, Samples::Samples1, ShaderModuleInfo, ShaderRepr,
    Subresource,
};

#[derive(ShaderRepr)]
#[sierra(std140)]
struct OffsetStride {
    offset: ivec2,
    stride: u32,
}

#[derive(Descriptors)]
#[sierra(capacity = 32)]
struct Rgb2RgbaDescriptors {
    #[sierra(buffer(texel = rgba8unorm, uniform), compute)]
    pixels: BufferView,

    #[sierra(image(storage, layout = General), compute)]
    image: Image,
}

#[allow(unused)]
#[derive(PipelineInput)]
struct Rgb2RgbaPipeline {
    #[sierra(set)]
    set: Rgb2RgbaDescriptors,

    #[sierra(push, compute)]
    offset_stride: OffsetStride,
}

pub(super) struct Rgb2RgbaUploader {
    layout: Rgb2RgbaPipelineLayout,
    descriptors: Mutex<Rgb2RgbaDescriptorsInstance>,
    pipeline: ComputePipeline,
}

impl Drop for Rgb2RgbaUploader {
    fn drop(&mut self) {
        self.descriptors.get_mut().clear();
    }
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

        let descriptors = layout.set.instance();

        Ok(Rgb2RgbaUploader {
            layout,
            pipeline,
            descriptors: Mutex::new(descriptors),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upload_synchronized<'a>(
        &self,
        device: &Device,
        image: &Image,
        offset: Offset3,
        extent: Extent3,
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

        let buffer_view = device.create_buffer_view(BufferViewInfo {
            format: Format::R8Unorm,
            offset: 0,
            size: buffer.info().size,
            buffer,
        })?;

        let staging_image = device.create_image(ImageInfo {
            extent: image.info().extent,
            format: Format::RGBA8Unorm,
            levels: 1,
            layers: 1,
            samples: Samples1,
            usage: ImageUsage::STORAGE | ImageUsage::TRANSFER_SRC,
        })?;

        encoder.image_barriers(
            PipelineStages::TOP_OF_PIPE,
            PipelineStages::COMPUTE_SHADER,
            &[ImageMemoryBarrier {
                image: &staging_image,
                old_layout: None,
                new_layout: Layout::General,
                old_access: Access::empty(),
                new_access: Access::SHADER_STORAGE_WRITE,
                family_transfer: None,
                range: Subresource {
                    aspect: AspectFlags::COLOR,
                    level: 0,
                    layer: 0,
                }
                .into(),
            }],
        );

        encoder.bind_compute_pipeline(&self.pipeline);

        {
            let mut descriptors = self.descriptors.lock();
            let updated = descriptors
                .update(
                    &Rgb2RgbaDescriptors {
                        pixels: buffer_view,
                        image: staging_image.clone(),
                    },
                    device,
                    encoder,
                )
                .map_err(|err| match err {
                    DescriptorsAllocationError::OutOfMemory { source } => source,
                    _ => {
                        tracing::error!("Unexpected error: {}", err);
                        OutOfMemory
                    }
                })?;

            encoder.bind_compute_descriptors(&self.layout, updated);
        }

        encoder.push_constants(
            &self.layout,
            &OffsetStride {
                offset: ivec2::from([offset.x, offset.y]),
                stride: row_length,
            },
        );
        encoder.dispatch(extent.width, extent.height, extent.depth);

        let subresource = Subresource {
            aspect: AspectFlags::COLOR,
            level: 0,
            layer: 0,
        };

        encoder.image_barriers(
            PipelineStages::COMPUTE_SHADER,
            PipelineStages::TRANSFER,
            &[ImageMemoryBarrier {
                image: &staging_image,
                old_layout: Some(Layout::General),
                new_layout: Layout::TransferSrcOptimal,
                old_access: Access::SHADER_STORAGE_WRITE,
                new_access: Access::TRANSFER_READ,
                family_transfer: None,
                range: subresource.into(),
            }],
        );

        encoder.copy_image(
            &staging_image,
            Layout::TransferSrcOptimal,
            image,
            Layout::TransferDstOptimal,
            &[ImageCopy {
                src_subresource: subresource.into(),
                src_offset: Offset3::zeros(),
                dst_subresource: subresource.into(),
                dst_offset: Offset3::zeros(),
                extent,
            }],
        );

        Ok(())
    }
}
