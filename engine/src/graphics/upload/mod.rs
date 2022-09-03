use std::{convert::TryFrom, mem::size_of_val};

use bytemuck::Pod;
use scoped_arena::Scope;
use sierra::{
    Access, Buffer, BufferCopy, BufferImageCopy, BufferInfo, BufferUsage, Device, Encoder, Extent3,
    Format, Image, ImageMemoryBarrier, Layout, Offset3, OutOfMemory, PipelineStages, Queue,
    SubresourceLayers,
};

use super::UploadImage;

mod rgb2rgba;

pub struct Uploader {
    buffer_uploads: Vec<BufferUpload>,
    image_uploads: Vec<ImageUpload>,

    rgb2rgba: rgb2rgba::Rgb2RgbaUploader,
}

impl Uploader {
    pub fn new(device: &Device) -> Result<Self, OutOfMemory> {
        Ok(Uploader {
            buffer_uploads: Vec::new(),
            image_uploads: Vec::new(),

            rgb2rgba: rgb2rgba::Rgb2RgbaUploader::new(device)?,
        })
    }

    pub fn upload_buffer<T>(
        &mut self,
        device: &Device,
        buffer: &Buffer,
        offset: u64,
        data: &[T],
    ) -> Result<(), OutOfMemory>
    where
        T: Pod,
    {
        if data.is_empty() {
            return Ok(());
        }

        let staging = device.create_buffer_static(
            BufferInfo {
                align: 15,
                size: size_of_val(data) as u64,
                usage: BufferUsage::TRANSFER_SRC,
            },
            data,
        )?;

        self.buffer_uploads.push(BufferUpload {
            staging,
            buffer: buffer.clone(),
            offset,
            old_access: Access::all(),
            new_access: Access::all(),
        });

        Ok(())
    }

    pub fn upload_buffer_with<'a, T>(
        &self,
        device: &Device,
        buffer: &'a Buffer,
        offset: u64,
        data: &'a [T],
        encoder: &mut Encoder<'a>,
    ) -> Result<(), OutOfMemory>
    where
        T: Pod,
    {
        const UPDATE_LIMIT: usize = 16384;

        assert_eq!(
            size_of_val(data) & 3,
            0,
            "Buffer uploading data size must be a multiple of 4"
        );

        if data.is_empty() {
            return Ok(());
        }

        if size_of_val(data) <= UPDATE_LIMIT {
            encoder.update_buffer(buffer, offset, data);
        } else {
            let staging = device.create_buffer_static(
                BufferInfo {
                    align: 15,
                    size: size_of_val(data) as u64,
                    usage: BufferUsage::TRANSFER_SRC,
                },
                data,
            )?;

            encoder.copy_buffer(
                &staging,
                buffer,
                &[BufferCopy {
                    src_offset: 0,
                    dst_offset: offset,
                    size: size_of_val(data) as u64,
                }],
            );
        }

        Ok(())
    }

    pub fn upload_image<T>(
        &mut self,
        device: &Device,
        upload: UploadImage,
        data: &[T],
    ) -> Result<(), OutOfMemory>
    where
        T: Pod,
    {
        let staging_usage = if upload.format == upload.image.info().format {
            BufferUsage::TRANSFER_SRC
        } else {
            BufferUsage::UNIFORM_TEXEL
        };

        let staging = device.create_buffer_static(
            BufferInfo {
                align: 15,
                size: u64::try_from(size_of_val(data)).map_err(|_| OutOfMemory)?,
                usage: staging_usage,
            },
            data,
        )?;

        self.image_uploads.push(ImageUpload {
            image: upload.image.clone(),
            offset: upload.offset,
            extent: upload.extent,
            layers: upload.layers,
            old_layout: upload.old_layout,
            new_layout: upload.new_layout,
            old_access: upload.old_access,
            new_access: upload.new_access,
            staging,
            format: upload.format,
            row_length: upload.row_length,
            image_height: upload.image_height,
        });

        Ok(())
    }

    pub fn upload_image_with<'a, T>(
        &self,
        device: &Device,
        upload: UploadImage,
        data: &[T],
        encoder: &mut Encoder<'a>,
    ) -> Result<(), OutOfMemory>
    where
        T: Pod,
    {
        let staging_usage = if upload.format == upload.image.info().format {
            BufferUsage::TRANSFER_SRC
        } else {
            BufferUsage::UNIFORM_TEXEL
        };

        let staging = device.create_buffer_static(
            BufferInfo {
                align: 15,
                size: u64::try_from(size_of_val(data)).map_err(|_| OutOfMemory)?,
                usage: staging_usage,
            },
            data,
        )?;

        encoder.image_barriers(
            PipelineStages::TOP_OF_PIPE,
            PipelineStages::TRANSFER,
            &[ImageMemoryBarrier {
                image: upload.image,
                old_layout: upload.old_layout,
                new_layout: Layout::TransferDstOptimal,
                old_access: upload.old_access,
                new_access: Access::TRANSFER_WRITE,
                family_transfer: None,
                range: upload.layers.into(),
            }],
        );

        match (upload.format, upload.image.info().format) {
            (from, to) if from == to => encoder.copy_buffer_to_image(
                &staging,
                upload.image,
                Layout::TransferDstOptimal,
                &[BufferImageCopy {
                    buffer_offset: 0,
                    buffer_row_length: upload.row_length,
                    buffer_image_height: upload.image_height,
                    image_subresource: upload.layers,
                    image_offset: upload.offset,
                    image_extent: upload.extent,
                }],
            ),
            (Format::RGB8Unorm, Format::RGBA8Unorm) => {
                self.rgb2rgba.upload_synchronized(
                    device,
                    upload.image,
                    upload.offset,
                    upload.extent,
                    staging.clone(),
                    upload.row_length,
                    upload.image_height,
                    encoder,
                )?;
            }
            (Format::RGB8Srgb, Format::RGBA8Srgb) => {
                self.rgb2rgba.upload_synchronized(
                    device,
                    upload.image,
                    upload.offset,
                    upload.extent,
                    staging.clone(),
                    upload.row_length,
                    upload.image_height,
                    encoder,
                )?;
            }
            (from, to) => {
                panic!("Uploading from '{:?}' to '{:?}' is unimplemented", from, to)
            }
        }

        encoder.image_barriers(
            PipelineStages::TRANSFER,
            PipelineStages::ALL_COMMANDS,
            &[ImageMemoryBarrier {
                image: upload.image,
                old_layout: Some(Layout::TransferDstOptimal),
                new_layout: upload.new_layout,
                old_access: Access::TRANSFER_WRITE,
                new_access: upload.new_access,
                family_transfer: None,
                range: upload.layers.into(),
            }],
        );

        Ok(())
    }

    pub fn flush_uploads(
        &mut self,
        device: &Device,
        queue: &mut Queue,
        scope: &Scope<'_>,
    ) -> Result<(), OutOfMemory> {
        if self.buffer_uploads.is_empty() && self.image_uploads.is_empty() {
            return Ok(());
        }

        let mut encoder = queue.create_encoder(scope)?;

        if !self.buffer_uploads.is_empty() {
            tracing::debug!("Uploading buffers");

            let mut old_access = Access::empty();
            let mut new_access = Access::empty();

            for upload in &self.buffer_uploads {
                old_access |= upload.old_access;
                new_access |= upload.new_access;
            }

            encoder.memory_barrier(
                PipelineStages::ALL_COMMANDS,
                old_access,
                PipelineStages::TRANSFER,
                Access::TRANSFER_WRITE,
            );

            for upload in &self.buffer_uploads {
                encoder.copy_buffer(
                    &upload.staging,
                    &upload.buffer,
                    &[BufferCopy {
                        src_offset: 0,
                        dst_offset: upload.offset,
                        size: upload.staging.info().size,
                    }],
                );
            }

            encoder.memory_barrier(
                PipelineStages::TRANSFER,
                Access::TRANSFER_WRITE,
                PipelineStages::ALL_COMMANDS,
                new_access,
            );
        }

        if !self.image_uploads.is_empty() {
            tracing::debug!("Uploading images");

            let mut images = Vec::with_capacity_in(self.image_uploads.len(), scope);

            for upload in &self.image_uploads {
                images.push(ImageMemoryBarrier {
                    image: &upload.image,
                    old_layout: upload.old_layout,
                    new_layout: Layout::TransferDstOptimal,
                    old_access: upload.old_access,
                    new_access: Access::TRANSFER_WRITE,
                    family_transfer: None,
                    range: upload.layers.into(),
                });
            }

            let images_len = images.len();

            encoder.image_barriers(
                PipelineStages::TOP_OF_PIPE,
                PipelineStages::TRANSFER,
                images.leak(),
            );

            for upload in &self.image_uploads {
                match (upload.format, upload.image.info().format) {
                    (from, to) if from == to => encoder.copy_buffer_to_image(
                        &upload.staging,
                        &upload.image,
                        Layout::TransferDstOptimal,
                        &[BufferImageCopy {
                            buffer_offset: 0,
                            buffer_row_length: upload.row_length,
                            buffer_image_height: upload.image_height,
                            image_subresource: upload.layers,
                            image_offset: upload.offset,
                            image_extent: upload.extent,
                        }],
                    ),
                    (Format::RGB8Unorm, Format::RGBA8Unorm) => {
                        self.rgb2rgba.upload_synchronized(
                            device,
                            &upload.image,
                            Offset3::zeros(),
                            upload.image.info().extent.into_3d(),
                            upload.staging.clone(),
                            upload.row_length,
                            upload.image_height,
                            &mut encoder,
                        )?;
                    }
                    (Format::RGB8Srgb, Format::RGBA8Srgb) => {
                        self.rgb2rgba.upload_synchronized(
                            device,
                            &upload.image,
                            Offset3::zeros(),
                            upload.image.info().extent.into_3d(),
                            upload.staging.clone(),
                            upload.row_length,
                            upload.image_height,
                            &mut encoder,
                        )?;
                    }
                    (from, to) => {
                        panic!("Uploading from '{:?}' to '{:?}' is unimplemented", from, to)
                    }
                }
            }

            let mut images = Vec::with_capacity_in(images_len, scope);

            for upload in &self.image_uploads {
                images.push(ImageMemoryBarrier {
                    image: &upload.image,
                    old_layout: Some(Layout::TransferDstOptimal),
                    new_layout: upload.new_layout,
                    old_access: Access::TRANSFER_WRITE,
                    new_access: upload.new_access,
                    family_transfer: None,
                    range: upload.layers.into(),
                });
            }

            encoder.image_barriers(
                PipelineStages::TRANSFER,
                PipelineStages::ALL_COMMANDS,
                images.leak(),
            );
        }

        queue.submit(&mut [], Some(encoder.finish()), &mut [], None, scope);

        self.buffer_uploads.clear();
        self.image_uploads.clear();
        Ok(())
    }
}

struct BufferUpload {
    staging: Buffer,
    buffer: Buffer,
    offset: u64,
    old_access: Access,
    new_access: Access,
}

struct ImageUpload {
    image: Image,
    offset: Offset3,
    extent: Extent3,
    layers: SubresourceLayers,
    old_layout: Option<Layout>,
    new_layout: Layout,
    old_access: Access,
    new_access: Access,
    staging: Buffer,
    format: Format,
    row_length: u32,
    image_height: u32,
}
