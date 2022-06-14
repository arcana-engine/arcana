//! Built-in arcana graphics.
//!
//! This module depends on `sierra` crate.
//! Graphics backend agnostic code should live outside this module.

#[macro_export]
/// Macro to simplify building of VertexLayout in const context.
macro_rules! vertex_location {
    ($offset:ident, $elem:ty as $semantics:literal) => {
        VertexLocation {
            format: <$elem as $crate::graphics::FormatElement>::FORMAT,
            semantics: $crate::graphics::Semantics::new($semantics),
            offset: {
                let offset = $offset;
                #[allow(unused_assignments)]
                {
                    $offset += ::core::mem::size_of::<$elem>() as u32;
                }
                offset
            },
        }
    };

    ($offset:ident, $va:ty) => {
        VertexLocation {
            format: <$va as $crate::graphics::VertexAttribute>::FORMAT,
            semantics: <$va as $crate::graphics::VertexAttribute>::SEMANTICS,
            offset: {
                let offset = $offset;
                #[allow(unused_assignments)]
                {
                    $offset += ::core::mem::size_of::<$va>() as u32;
                }
                offset
            },
        }
    };
}

#[macro_export]
macro_rules! define_vertex_attribute {
    ($(
        $(#[$meta:meta])*
        $vis:vis struct $va:ident as $semantics:literal ($fvis:vis $ft:ty);
    )*) => {$(
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        #[repr(transparent)]
        $vis struct $va($fvis $ft);

        const _: () = $crate::assert_pod::<$ft>();

        unsafe impl $crate::bytemuck::Zeroable for $va {}
        unsafe impl $crate::bytemuck::Pod for $va {}

        impl $crate::graphics::VertexAttribute for $va {
            const FORMAT: $crate::sierra::Format = <$ft as $crate::graphics::FormatElement>::FORMAT;
            const SEMANTICS: $crate::graphics::Semantics = $crate::graphics::Semantics::new($semantics);
        }

        impl<T> From<T> for $va where T: Into<$ft> {
            #[inline]
            fn from(t: T) -> $va {
                $va(t.into())
            }
        }
    )*};

    ($(
        $(#[$meta:meta])*
        $vis:vis struct $va:ident as $semantics:tt ($fvis:vis $ft:ty);
    )*) => {$(
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        #[repr(transparent)]
        $vis struct $va($fvis $ft);

        const _: () = $crate::assert_pod::<$ft>();

        unsafe impl $crate::bytemuck::Zeroable for $va {}
        unsafe impl $crate::bytemuck::Pod for $va {}

        impl $crate::graphics::VertexAttribute for $va {
            const FORMAT: $crate::sierra::Format = <$ft as $crate::graphics::FormatElement>::FORMAT;
            const SEMANTICS: $crate::graphics::Semantics = $semantics;
        }

        impl<T> From<T> for $va where T: Into<$ft> {
            #[inline]
            fn from(t: T) -> $va {
                $va(t.into())
            }
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

        $(
            const _: () = $crate::assert_pod::<$vat>();
        )*

        unsafe impl $crate::bytemuck::Zeroable for $vt {}
        unsafe impl $crate::bytemuck::Pod for $vt {}

        impl $crate::graphics::VertexType for $vt {
            const LOCATIONS: &'static [$crate::graphics::VertexLocation] = {
                let mut offset = 0;
                $(
                    let $van = $crate::vertex_location!(offset, $vat $(as $semantics)? );
                )*
                &[$($van,)*]
            };
            const RATE: $crate::graphics::VertexInputRate = $crate::graphics::VertexInputRate::$rate;
        }
    )*};
}

pub mod node;
pub mod renderer;

mod format;
mod material;
mod scale;
mod texture;
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
    AccessFlags, Buffer, BufferInfo, CommandBuffer, CreateSurfaceError, Device, Encoder, Extent3d,
    Fence, Format, Image, ImageInfo, ImageUsage, Layout, Offset3d, OutOfMemory, PipelineStageFlags,
    PresentOk, Queue, Semaphore, SingleQueueQuery, SubresourceLayers, Surface, SwapchainImage,
};

pub use sierra::VertexInputRate;

use self::upload::Uploader;
pub use self::{format::*, material::*, scale::*, texture::*, vertex::*};

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
                sierra::Feature::ShaderStorageImageDynamicIndexing,
                sierra::Feature::ShaderStorageImageNonUniformIndexing,
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
    ) -> Result<(), OutOfMemory>
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
    ) -> Result<(), OutOfMemory>
    where
        T: Pod,
    {
        self.uploader
            .upload_buffer_with(&self.device, buffer, offset, data, encoder)
    }

    #[inline]
    #[tracing::instrument(skip(self, data))]
    pub fn upload_image<T>(&mut self, upload: UploadImage, data: &[T]) -> Result<(), OutOfMemory>
    where
        T: Pod,
    {
        self.uploader.upload_image(&self.device, upload, data)
    }

    #[tracing::instrument(skip(self, data))]
    pub fn upload_image_with<'a, T>(
        &self,
        upload: UploadImage,
        data: &[T],
        encoder: &mut Encoder<'a>,
    ) -> Result<(), OutOfMemory>
    where
        T: Pod,
    {
        self.uploader
            .upload_image_with(&self.device, upload, data, encoder)
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
        let buffer = self.device.create_buffer(info)?;
        self.upload_buffer(&buffer, 0, data)?;
        Ok(buffer)
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
    ) -> Result<Image, OutOfMemory>
    where
        T: Pod,
    {
        info.usage |= ImageUsage::TRANSFER_DST;
        let layers = SubresourceLayers::all_layers(&info, 0);
        let image = self.device.create_image(info)?;
        self.upload_image(
            UploadImage {
                image: &image,
                offset: Offset3d::ZERO,
                extent: info.extent.into_3d(),
                layers,
                old_layout: None,
                new_layout: layout,
                old_access: AccessFlags::empty(),
                new_access: AccessFlags::all(),
                format,
                row_length,
                image_height,
            },
            data,
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

impl<T> Default for SparseDescriptors<T> {
    #[inline]
    fn default() -> Self {
        SparseDescriptors::new()
    }
}

impl<T> SparseDescriptors<T> {
    #[inline]
    pub fn new() -> Self {
        SparseDescriptors {
            resources: HashMap::new(),
            bitset: BitEmpty::empty(),
            next: 0,
        }
    }

    pub fn index(&mut self, resource: T) -> (u32, bool)
    where
        T: Hash + Eq,
    {
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

#[derive(Debug)]
pub struct UploadImage<'a> {
    pub image: &'a Image,
    pub offset: Offset3d,
    pub extent: Extent3d,
    pub layers: SubresourceLayers,
    pub old_layout: Option<Layout>,
    pub new_layout: Layout,
    pub old_access: AccessFlags,
    pub new_access: AccessFlags,
    pub format: Format,
    pub row_length: u32,
    pub image_height: u32,
}
