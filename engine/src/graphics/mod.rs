//! Built-in arcana graphics.
//!
//! This module depends on `sierra` crate.
//! Graphics backend agnostic code should live outside this module.

#[macro_export]
/// Macro to simplify building of VertexLayout in const context.
macro_rules! vertex_location {
    ($offset:ident, $elem:ty as $semantics:literal) => {
        #[allow(unused_assignments)]
        VertexLocation {
            format: <$elem as $crate::graphics::FormatElement>::FORMAT,
            semantics: $crate::graphics::Semantics::new($semantics),
            offset: {
                let o = $offset;
                $offset += ::core::mem::size_of::<$elem>() as u32;
                o
            },
        }
    };

    ($offset:ident, $va:ty) => {
        #[allow(unused_assignments)]
        VertexLocation {
            format: <$va as $crate::graphics::VertexAttribute>::FORMAT,
            semantics: <$va as $crate::graphics::VertexAttribute>::SEMANTICS,
            offset: {
                let o = $offset;
                $offset += ::core::mem::size_of::<$va>() as u32;
                o
            },
        }
    };
}

#[macro_export]
macro_rules! define_vertex_attribute {
    ($(
        $(#[$meta:meta])*
        $vis:vis struct $va:ident as $semantics:tt ($fvis:vis $ft:ty);
    )*) => {$(
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        #[repr(transparent)]
        $vis struct $va($fvis $ft);

        unsafe impl bytemuck::Zeroable for $va {}
        unsafe impl bytemuck::Pod for $va {}

        impl $crate::graphics::VertexAttribute for $va {
            const FORMAT: $crate::sierra::Format = <$ft as $crate::graphics::FormatElement>::FORMAT;
            const SEMANTICS: $crate::graphics::Semantics = $semantics;
        }
    )*};
}

#[macro_export]
macro_rules! define_vertex_type {
    ($(
        $(#[$meta:meta])*
        $vis:vis struct $vt:ident as $rate:ident {
            $( $van:ident: $vat:ty $(as $semantics:literal)? ),*
            $(,)?
        }
    )*) => {$(
        $(#[$meta])*
        #[repr(C)]
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        $vis struct $vt {
            $( $van: $vat, )*
        }

        unsafe impl bytemuck::Zeroable for $vt {}
        unsafe impl bytemuck::Pod for $vt {}

        impl $crate::graphics::VertexType for $vt {
            const LOCATIONS: &'static [$crate::graphics::VertexLocation] = {
                let mut offset = 0;
                &[$(
                    $crate::vertex_location!(offset, $vat as $semantics ),
                ),*]
            };
            const RATE: $crate::graphics::VertexInputRate = $crate::graphics::VertexInputRate::$rate;
        }
    )*};
}

mod format;
pub mod node;
pub mod renderer;
mod scale;
mod upload;
mod vertex;

#[cfg(feature = "3d")]
mod mesh;

use std::{
    collections::hash_map::{Entry, HashMap},
    hash::Hash,
    ops::Deref,
};

use bitsetium::{BitEmpty, BitSearch, BitUnset, Bits1024};
use bytemuck::Pod;
use raw_window_handle::HasRawWindowHandle;
use scoped_arena::Scope;
use sierra::{
    AccessFlags, Buffer, BufferInfo, CommandBuffer, CreateImageError, CreateSurfaceError, Device,
    Encoder, Extent3d, Fence, Format, Image, ImageInfo, ImageUsage, Layout, MapError, Offset3d,
    OutOfMemory, PipelineStageFlags, PresentOk, Queue, Semaphore, SingleQueueQuery,
    SubresourceLayers, Surface, SwapchainImage,
};

use self::upload::Uploader;
pub use self::{format::*, renderer::*, scale::*, vertex::*};

#[cfg(feature = "3d")]
pub use self::mesh::*;

/// Graphics context.
/// Combines device and single queue.
/// Suitable for not too complex graphics tasks.
pub struct Graphics {
    uploader: Uploader,
    queue: Queue,
    device: Device,
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
                sierra::Feature::ScalarBlockLayout,
            ],
            SingleQueueQuery::GRAPHICS,
        )?;

        Ok(Graphics {
            uploader: Uploader::new(&device)?,
            device,
            queue,
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

    #[inline]
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
        self.uploader
            .upload_buffer(&self.device, buffer, offset, data)
    }

    #[inline]
    #[tracing::instrument(skip(self, data))]
    pub fn upload_buffer_with<'a, T>(
        &self,
        buffer: &'a Buffer,
        offset: u64,
        data: &'a [T],
        encoder: &mut Encoder<'a>,
    ) -> Result<(), MapError>
    where
        T: Pod,
    {
        self.uploader
            .upload_buffer_with(&self.device, buffer, offset, data, encoder)
    }

    #[inline]
    #[tracing::instrument(skip(self, data))]
    pub fn upload_image<T>(
        &mut self,
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
        self.uploader.upload_image(
            &self.device,
            image,
            layers,
            old_layout,
            new_layout,
            old_access,
            new_access,
            data,
            format,
            row_length,
            image_height,
        )
    }

    #[tracing::instrument(skip(self, data))]
    pub fn upload_image_with<'a, T>(
        &self,
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
        self.uploader.upload_image_with(
            &self.device,
            image,
            layers,
            old_layout,
            new_layout,
            old_access,
            new_access,
            offset,
            extent,
            data,
            format,
            row_length,
            image_height,
            encoder,
        )
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
        data: &[T],
        format: Format,
        row_length: u32,
        image_height: u32,
    ) -> Result<Image, CreateImageError>
    where
        T: Pod,
    {
        info.usage |= ImageUsage::TRANSFER_DST;
        let subresource = SubresourceLayers::all_layers(&info, 0);
        let image = self.device.create_image(info)?;
        self.upload_image(
            &image,
            subresource,
            None,
            layout,
            AccessFlags::empty(),
            AccessFlags::all(),
            data,
            format,
            row_length,
            image_height,
        )?;
        Ok(image)
    }

    pub fn create_encoder<'a>(&mut self, scope: &'a Scope<'a>) -> Result<Encoder<'a>, OutOfMemory> {
        self.queue.create_encoder(scope)
    }

    pub fn submit(
        &mut self,
        wait: &mut [(PipelineStageFlags, &mut Semaphore)],
        cbufs: impl IntoIterator<Item = CommandBuffer>,
        signal: &mut [&mut Semaphore],
        fence: Option<&mut Fence>,
        scope: &Scope<'_>,
    ) -> Result<(), OutOfMemory> {
        self.flush_uploads(scope)?;
        self.queue.submit(wait, cbufs, signal, fence, scope);
        Ok(())
    }

    pub fn present(&mut self, image: SwapchainImage) -> Result<PresentOk, OutOfMemory> {
        self.queue.present(image)
    }

    fn flush_uploads(&mut self, scope: &Scope<'_>) -> Result<(), OutOfMemory> {
        self.uploader
            .flush_uploads(&self.device, &mut self.queue, scope)
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.wait_idle();
        }
    }
}

impl Deref for Graphics {
    type Target = Device;

    #[inline(always)]
    fn deref(&self) -> &Device {
        &self.device
    }
}

pub struct SparseDescriptors<T> {
    resources: HashMap<T, u32>,
    bitset: Bits1024,
    next: u32,
}

impl<T> SparseDescriptors<T>
where
    T: Hash + Eq,
{
    pub fn new() -> Self {
        SparseDescriptors {
            resources: HashMap::new(),
            bitset: BitEmpty::empty(),
            next: 0,
        }
    }

    pub fn index(&mut self, resource: T) -> (u32, bool) {
        match self.resources.entry(resource) {
            Entry::Occupied(entry) => (*entry.get(), false),
            Entry::Vacant(entry) => {
                if let Some(index) = self.bitset.find_first_set(0) {
                    self.bitset.unset(index);
                    (*entry.insert(index as u32), true)
                } else {
                    self.next += 1;
                    (*entry.insert(self.next - 1), true)
                }
            }
        }
    }
}
