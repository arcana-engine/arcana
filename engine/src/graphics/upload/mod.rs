use std::{convert::TryFrom, mem::size_of_val};

use bytemuck::Pod;
use scoped_arena::Scope;
use sierra::{
    AccessFlags, Buffer, BufferCopy, BufferImageCopy, BufferInfo, BufferUsage, Device, Encoder,
    Extent3d, Format, Image, ImageMemoryBarrier, Layout, MapError, Offset3d, OutOfMemory,
    PipelineStageFlags, Queue, SubresourceLayers,
};

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
    ) -> Result<(), MapError>
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
            old_access: AccessFlags::all(),
            new_access: AccessFlags::all(),
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
    ) -> Result<(), MapError>
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

            let staging = encoder.scope().to_scope(staging);

            encoder.copy_buffer(
                &*staging,
                buffer,
                encoder.scope().to_scope([BufferCopy {
                    src_offset: 0,
                    dst_offset: offset,
                    size: size_of_val(data) as u64,
                }]),
            );
        }

        Ok(())
    }

    pub fn upload_image<T>(
        &mut self,
        device: &Device,
        image: &Image,
        layers: SubresourceLayers,
        old_layout: Option<Layout>,
        new_layout: Layout,
        old_access: AccessFlags,
        new_access: AccessFlags,
        data: &[T],
        format: Format,
        row_length: u32,
        image_height: u32,
    ) -> Result<(), OutOfMemory>
    where
        T: Pod,
    {
        let staging_usage = if format == image.info().format {
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
            image: image.clone(),
            layers,
            old_layout,
            new_layout,
            old_access,
            new_access,
            staging,
            format,
            row_length,
            image_height,
        });

        Ok(())
    }

    pub fn upload_image_with<'a, T>(
        &self,
        device: &Device,
        image: &Image,
        layers: SubresourceLayers,
        old_layout: Option<Layout>,
        new_layout: Layout,
        old_access: AccessFlags,
        new_access: AccessFlags,
        offset: Offset3d,
        extent: Extent3d,
        data: &[T],
        format: Format,
        row_length: u32,
        image_height: u32,
        encoder: &mut Encoder<'a>,
    ) -> Result<(), MapError>
    where
        T: Pod,
    {
        let staging_usage = if format == image.info().format {
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

        let scope = encoder.scope();

        let image = &*scope.to_scope(image.clone());

        encoder.image_barriers(
            PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::TRANSFER,
            scope.to_scope([ImageMemoryBarrier {
                image,
                old_layout,
                new_layout: Layout::TransferDstOptimal,
                old_access,
                new_access: AccessFlags::TRANSFER_WRITE,
                family_transfer: None,
                range: layers.into(),
            }]),
        );

        // encoder.copy_buffer_to_image(
        //     scope.to_scope(staging),
        //     scope.to_scope(image.clone()),
        //     Layout::TransferDstOptimal,
        //     scope.to_scope([BufferImageCopy {
        //         buffer_offset: 0,
        //         buffer_row_length: row_length,
        //         buffer_image_height: image_height,
        //         image_subresource: layers,
        //         image_offset: offset,
        //         image_extent: extent,
        //     }]),
        // );

        match (format, image.info().format) {
            (from, to) if from == to => encoder.copy_buffer_to_image(
                scope.to_scope(staging),
                scope.to_scope(image.clone()),
                Layout::TransferDstOptimal,
                scope.to_scope([BufferImageCopy {
                    buffer_offset: 0,
                    buffer_row_length: row_length,
                    buffer_image_height: image_height,
                    image_subresource: layers,
                    image_offset: offset,
                    image_extent: extent,
                }]),
            ),
            (Format::RGB8Unorm, Format::RGBA8Unorm) => {
                self.rgb2rgba.upload_synchronized(
                    device,
                    scope.to_scope(image.clone()),
                    offset,
                    extent,
                    staging.clone(),
                    row_length,
                    image_height,
                    encoder,
                )?;
            }
            (Format::RGB8Srgb, Format::RGBA8Srgb) => {
                self.rgb2rgba.upload_synchronized(
                    device,
                    scope.to_scope(image.clone()),
                    offset,
                    extent,
                    staging.clone(),
                    row_length,
                    image_height,
                    encoder,
                )?;
            }
            (from, to) => {
                panic!("Uploading from '{:?}' to '{:?}' is unimplemented", from, to)
            }
        }

        encoder.image_barriers(
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::ALL_COMMANDS,
            scope.to_scope([ImageMemoryBarrier {
                image,
                old_layout: Some(Layout::TransferDstOptimal),
                new_layout,
                old_access: AccessFlags::TRANSFER_WRITE,
                new_access,
                family_transfer: None,
                range: layers.into(),
            }]),
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

            let mut old_access = AccessFlags::empty();
            let mut new_access = AccessFlags::empty();

            for upload in &self.buffer_uploads {
                old_access |= upload.old_access;
                new_access |= upload.new_access;
            }

            encoder.memory_barrier(
                PipelineStageFlags::ALL_COMMANDS,
                old_access,
                PipelineStageFlags::TRANSFER,
                AccessFlags::TRANSFER_WRITE,
            );

            for upload in &self.buffer_uploads {
                encoder.copy_buffer(
                    &upload.staging,
                    &upload.buffer,
                    scope.to_scope([BufferCopy {
                        src_offset: 0,
                        dst_offset: upload.offset,
                        size: upload.staging.info().size,
                    }]),
                );
            }

            encoder.memory_barrier(
                PipelineStageFlags::TRANSFER,
                AccessFlags::TRANSFER_WRITE,
                PipelineStageFlags::ALL_COMMANDS,
                new_access,
            );
        }

        if !self.image_uploads.is_empty() {
            tracing::debug!("Uploading images");

            let mut images = Vec::with_capacity_in(self.image_uploads.len(), scope);

            for upload in &self.image_uploads {
                images.push(ImageMemoryBarrier {
                    image: scope.to_scope(upload.image.clone()),
                    old_layout: upload.old_layout,
                    new_layout: Layout::TransferDstOptimal,
                    old_access: upload.old_access,
                    new_access: AccessFlags::TRANSFER_WRITE,
                    family_transfer: None,
                    range: upload.layers.into(),
                });
            }

            let images_len = images.len();

            encoder.image_barriers(
                PipelineStageFlags::TOP_OF_PIPE,
                PipelineStageFlags::TRANSFER,
                images.leak(),
            );

            for upload in &self.image_uploads {
                match (upload.format, upload.image.info().format) {
                    (from, to) if from == to => encoder.copy_buffer_to_image(
                        &upload.staging,
                        &upload.image,
                        Layout::TransferDstOptimal,
                        scope.to_scope([BufferImageCopy {
                            buffer_offset: 0,
                            buffer_row_length: upload.row_length,
                            buffer_image_height: upload.image_height,
                            image_subresource: upload.layers,
                            image_offset: Offset3d::ZERO,
                            image_extent: upload.image.info().extent.into_3d(),
                        }]),
                    ),
                    (Format::RGB8Unorm, Format::RGBA8Unorm) => {
                        self.rgb2rgba.upload_synchronized(
                            device,
                            &upload.image,
                            Offset3d::ZERO,
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
                            Offset3d::ZERO,
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
                    image: scope.to_scope(upload.image.clone()),
                    old_layout: Some(Layout::TransferDstOptimal),
                    new_layout: upload.new_layout,
                    old_access: AccessFlags::TRANSFER_WRITE,
                    new_access: upload.new_access,
                    family_transfer: None,
                    range: upload.layers.into(),
                });
            }

            encoder.image_barriers(
                PipelineStageFlags::TRANSFER,
                PipelineStageFlags::ALL_COMMANDS,
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
    old_access: AccessFlags,
    new_access: AccessFlags,
}

struct ImageUpload {
    image: Image,
    layers: SubresourceLayers,
    old_layout: Option<Layout>,
    new_layout: Layout,
    old_access: AccessFlags,
    new_access: AccessFlags,
    staging: Buffer,
    format: Format,
    row_length: u32,
    image_height: u32,
}
