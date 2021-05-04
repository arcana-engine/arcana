mod material;
mod mesh;
pub mod node;
pub mod renderer;
mod scale;
mod sprite;
mod texture;
mod vertex;

use {
    bumpalo::{collections::Vec as BVec, Bump},
    bytemuck::Pod,
    raw_window_handle::HasRawWindowHandle,
    sierra::{
        AccessFlags, Buffer, BufferCopy, BufferImageCopy, BufferInfo, BufferUsage, CommandBuffer,
        CreateImageError, CreateSurfaceError, Device, Encoder, Extent3d, Fence, Image, ImageInfo,
        ImageMemoryBarrier, ImageUsage, Layout, MapError, Offset3d, OutOfMemory,
        PipelineStageFlags, PresentOk, Queue, Semaphore, SingleQueueQuery, SubresourceLayers,
        SubresourceRange, Surface, SwapchainImage,
    },
    std::{convert::TryFrom as _, mem::size_of_val, ops::Deref},
};

pub use {
    self::{
        material::Material,
        mesh::{Binding, BindingData, Indices, IndicesData, Mesh, MeshBuilder, MeshData, PoseMesh},
        renderer::{Renderer, RendererContext},
        scale::Scale,
        sprite::{Sprite, AABB},
        texture::Texture,
        vertex::{
            Color, Joints, Normal3, Position3, Position3Color, Position3UV, PositionNormal3,
            PositionNormal3Color, PositionNormal3UV, PositionNormalTangent3,
            PositionNormalTangent3UV, Semantics, Skin, Tangent3, Transformation3d, VertexLayout,
            VertexLocation, VertexType, Weights, UV,
        },
    },
    sierra::*,
};

/// Graphics context.
/// Combines device and single queue.
/// Suitable for not too complex graphics tasks.
pub struct Graphics {
    device: Device,
    queue: Queue,
    buffer_uploads: Vec<BufferUpload>,
    image_uploads: Vec<ImageUpload>,
}

impl Graphics {
    /// Create new instance of simple renderer.
    pub fn new() -> eyre::Result<Self> {
        let graphics = sierra::Graphics::get_or_init()?;

        let physical = graphics
            .devices()?
            .into_iter()
            .max_by_key(|d| d.info().kind)
            .ok_or_else(|| eyre::eyre!("Failed to find physical device"))?;

        let (device, queue) = physical.create_device(
            &[
                sierra::Feature::SurfacePresentation,
                sierra::Feature::ShaderSampledImageDynamicIndexing,
                sierra::Feature::ShaderSampledImageNonUniformIndexing,
                sierra::Feature::RuntimeDescriptorArray,
            ],
            SingleQueueQuery::GRAPHICS,
        )?;

        Ok(Graphics {
            device,
            queue,
            buffer_uploads: Vec::new(),
            image_uploads: Vec::new(),
        })
    }
}

impl Graphics {
    /// Returns newly created surface for a window.
    #[tracing::instrument(skip(self, window))]
    pub fn create_surface(
        &self,
        window: &impl HasRawWindowHandle,
    ) -> Result<Surface, CreateSurfaceError> {
        self.device.graphics().create_surface(window)
    }

    #[tracing::instrument(skip(self, data))]
    pub fn upload_buffer<T>(
        &mut self,
        buffer: &Buffer,
        offset: u64,
        data: &[T],
    ) -> Result<(), MapError>
    where
        T: Pod,
    {
        let staging = self.device.create_buffer_static(
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
            access: AccessFlags::all(),
        });

        Ok(())
    }

    #[tracing::instrument(skip(self, data))]
    pub fn upload_buffer_with<'a, T>(
        &self,
        buffer: &'a Buffer,
        offset: u64,
        data: &[T],
        encoder: &mut Encoder<'a>,
        staging_out: &'a mut Option<Buffer>,
    ) -> Result<(), MapError>
    where
        T: Pod,
    {
        let staging = self.device.create_buffer_static(
            BufferInfo {
                align: 15,
                size: size_of_val(data) as u64,
                usage: BufferUsage::TRANSFER_SRC,
            },
            data,
        )?;

        *staging_out = None;
        let staging = staging_out.get_or_insert(staging);

        encoder.copy_buffer(
            staging,
            buffer,
            &[BufferCopy {
                src_offset: 0,
                dst_offset: offset,
                size: staging.info().size,
            }],
        );

        encoder.memory_barrier(
            PipelineStageFlags::TRANSFER,
            AccessFlags::TRANSFER_WRITE,
            PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::all(),
        );

        Ok(())
    }

    #[tracing::instrument(skip(self, data))]
    pub fn upload_image<T>(
        &mut self,
        image: &Image,
        layout: Layout,
        row_length: u32,
        image_height: u32,
        subresource: SubresourceLayers,
        offset: Offset3d,
        extent: Extent3d,
        data: &[T],
    ) -> Result<(), OutOfMemory>
    where
        T: Pod,
    {
        let staging = self.device.create_buffer_static(
            BufferInfo {
                align: 15,
                size: u64::try_from(size_of_val(data)).map_err(|_| OutOfMemory)?,
                usage: BufferUsage::TRANSFER_SRC,
            },
            data,
        )?;

        self.image_uploads.push(ImageUpload {
            staging,
            image: image.clone(),
            access: AccessFlags::all(),
            layout,
            row_length,
            image_height,
            subresource,
            offset,
            extent,
        });

        Ok(())
    }

    #[tracing::instrument(skip(self, data))]
    pub fn create_fast_buffer_static<T>(
        &mut self,
        info: BufferInfo,
        data: &[T],
    ) -> Result<Buffer, OutOfMemory>
    where
        T: Pod,
    {
        let mut buffer = self.device.create_buffer(info)?;
        match self.upload_buffer(&mut buffer, 0, data) {
            Ok(()) => Ok(buffer),
            Err(MapError::OutOfMemory { .. }) => Err(OutOfMemory),
            _ => unreachable!(),
        }
    }

    #[tracing::instrument(skip(self, data))]
    pub fn create_image_static<T>(
        &mut self,
        mut info: ImageInfo,
        layout: Layout,
        row_length: u32,
        image_height: u32,
        data: &[T],
    ) -> Result<Image, CreateImageError>
    where
        T: Pod,
    {
        info.usage |= ImageUsage::TRANSFER_DST;
        let subresource = SubresourceLayers::all_layers(&info, 0);
        let image = self.device.create_image(info)?;
        self.upload_image(
            &image,
            layout,
            row_length,
            image_height,
            subresource,
            Offset3d::ZERO,
            info.extent.into_3d(),
            data,
        )?;
        Ok(image)
    }

    pub fn flush_uploads(&mut self, bump: &Bump) -> eyre::Result<()> {
        if self.buffer_uploads.is_empty() && self.image_uploads.is_empty() {
            return Ok(());
        }

        let mut encoder = self.queue.create_encoder(bump)?;

        if !self.buffer_uploads.is_empty() {
            tracing::debug!("Uploading buffers");

            let mut dst_acc = AccessFlags::empty();

            for upload in &self.buffer_uploads {
                encoder.copy_buffer(
                    &upload.staging,
                    &upload.buffer,
                    bump.alloc([BufferCopy {
                        src_offset: 0,
                        dst_offset: upload.offset,
                        size: upload.staging.info().size,
                    }]),
                );

                dst_acc |= upload.access;
            }

            if !dst_acc.is_empty() {
                encoder.memory_barrier(
                    PipelineStageFlags::TRANSFER,
                    AccessFlags::TRANSFER_WRITE,
                    PipelineStageFlags::ALL_COMMANDS,
                    dst_acc,
                );
            }
        }

        if !self.image_uploads.is_empty() {
            tracing::debug!("Uploading images");

            let mut images = BVec::with_capacity_in(self.image_uploads.len(), bump);

            for upload in &self.image_uploads {
                images.push(ImageMemoryBarrier {
                    image: bump.alloc(upload.image.clone()),
                    old_layout: None,
                    new_layout: Layout::TransferDstOptimal,
                    old_access: AccessFlags::empty(),
                    new_access: AccessFlags::TRANSFER_WRITE,
                    family_transfer: None,
                    range: SubresourceRange::whole(upload.image.info()),
                });
            }

            let images_len = images.len();

            encoder.image_barriers(
                PipelineStageFlags::TOP_OF_PIPE,
                PipelineStageFlags::TRANSFER,
                images.into_bump_slice(),
            );

            for upload in &self.image_uploads {
                encoder.copy_buffer_to_image(
                    &upload.staging,
                    &upload.image,
                    Layout::TransferDstOptimal,
                    bump.alloc([BufferImageCopy {
                        buffer_offset: 0,
                        buffer_row_length: upload.row_length,
                        buffer_image_height: upload.image_height,
                        image_subresource: upload.subresource,
                        image_offset: upload.offset,
                        image_extent: upload.extent,
                    }]),
                )
            }

            let mut images = BVec::with_capacity_in(images_len, bump);

            for upload in &self.image_uploads {
                images.push(ImageMemoryBarrier {
                    image: bump.alloc(upload.image.clone()),
                    old_layout: Some(Layout::TransferDstOptimal),
                    new_layout: upload.layout,
                    old_access: AccessFlags::TRANSFER_WRITE,
                    new_access: upload.access,
                    family_transfer: None,
                    range: SubresourceRange::whole(upload.image.info()),
                });
            }

            encoder.image_barriers(
                PipelineStageFlags::TRANSFER,
                PipelineStageFlags::ALL_COMMANDS,
                images.into_bump_slice(),
            );
        }

        self.queue
            .submit(&mut [], Some(encoder.finish()), &mut [], None, bump);

        self.buffer_uploads.clear();
        self.image_uploads.clear();
        Ok(())
    }

    pub fn create_encoder<'a>(&mut self, bump: &'a Bump) -> Result<Encoder<'a>, OutOfMemory> {
        self.queue.create_encoder(bump)
    }

    pub fn submit(
        &mut self,
        wait: &mut [(PipelineStageFlags, &mut Semaphore)],
        cbufs: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = CommandBuffer>>,
        signal: &mut [&mut Semaphore],
        fence: Option<&mut Fence>,
        bump: &Bump,
    ) {
        self.queue.submit(wait, cbufs, signal, fence, bump)
    }

    pub fn present(&mut self, image: SwapchainImage) -> Result<PresentOk, OutOfMemory> {
        self.queue.present(image)
    }
}

impl Deref for Graphics {
    type Target = Device;

    #[inline(always)]
    fn deref(&self) -> &Device {
        &self.device
    }
}

struct BufferUpload {
    staging: Buffer,
    buffer: Buffer,
    offset: u64,
    access: AccessFlags,
}

struct ImageUpload {
    staging: Buffer,
    image: Image,
    access: AccessFlags,
    layout: Layout,
    row_length: u32,
    image_height: u32,
    subresource: SubresourceLayers,
    offset: Offset3d,
    extent: Extent3d,
}
