use {
    crate::graphics::Graphics,
    goods::{Asset, AssetBuild, Loader},
    image::{load_from_memory, DynamicImage, GenericImageView as _, ImageError},
    sierra::{
        CreateImageError, ImageExtent, ImageInfo, ImageUsage, ImageView, ImageViewInfo, Layout,
        Samples1,
    },
    std::{
        borrow::BorrowMut,
        future::{ready, Ready},
    },
};

/// Image asset.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImageAsset(pub ImageView);

impl ImageAsset {
    pub fn into_inner(self) -> ImageView {
        self.0
    }
}

impl From<ImageAsset> for ImageView {
    fn from(asset: ImageAsset) -> Self {
        asset.0
    }
}

impl Asset for ImageAsset {
    type DecodeError = ImageError;
    type BuildError = CreateImageError;
    type Decoded = DynamicImage;
    type Fut = Ready<Result<DynamicImage, ImageError>>;

    fn decode(bytes: Box<[u8]>, _loader: &Loader) -> Self::Fut {
        ready(load_from_memory(&bytes).map_err(Into::into))
    }
}
impl<B> AssetBuild<B> for ImageAsset
where
    B: BorrowMut<Graphics>,
{
    fn build(image: DynamicImage, builder: &mut B) -> Result<Self, CreateImageError> {
        let image = image.to_rgba8();
        let image = image_view_from_dyn_image(
            &DynamicImage::ImageRgba8(image),
            false,
            builder.borrow_mut(),
        )?;

        Ok(ImageAsset(image))
    }
}

pub fn image_view_from_dyn_image(
    image: &DynamicImage,
    srgb: bool,
    graphics: &mut Graphics,
) -> Result<ImageView, CreateImageError> {
    use sierra::Format;

    let format = match (&image, srgb) {
        (DynamicImage::ImageLuma8(_), false) => Format::R8Unorm,
        (DynamicImage::ImageLumaA8(_), false) => Format::RG8Unorm,
        (DynamicImage::ImageRgb8(_), false) => Format::RGB8Unorm,
        (DynamicImage::ImageRgba8(_), false) => Format::RGBA8Unorm,
        (DynamicImage::ImageBgr8(_), false) => Format::BGR8Unorm,
        (DynamicImage::ImageBgra8(_), false) => Format::BGRA8Unorm,
        (DynamicImage::ImageLuma16(_), false) => Format::R16Unorm,
        (DynamicImage::ImageLumaA16(_), false) => Format::RG16Unorm,
        (DynamicImage::ImageRgb16(_), false) => Format::RGB16Unorm,
        (DynamicImage::ImageRgba16(_), false) => Format::RGBA16Unorm,

        (DynamicImage::ImageLuma8(_), true) => Format::R8Srgb,
        (DynamicImage::ImageLumaA8(_), true) => Format::RG8Srgb,
        (DynamicImage::ImageRgb8(_), true) => Format::RGB8Srgb,
        (DynamicImage::ImageRgba8(_), true) => Format::RGBA8Srgb,
        (DynamicImage::ImageBgr8(_), true) => Format::BGR8Srgb,
        (DynamicImage::ImageBgra8(_), true) => Format::BGRA8Srgb,
        (DynamicImage::ImageLuma16(_), true) => Format::R16Unorm,
        (DynamicImage::ImageLumaA16(_), true) => Format::RG16Unorm,
        (DynamicImage::ImageRgb16(_), true) => Format::RGB16Unorm,
        (DynamicImage::ImageRgba16(_), true) => Format::RGBA16Unorm,
    };

    let (w, h) = image.dimensions();

    let bytes8;
    let bytes16;

    let bytes = match image {
        DynamicImage::ImageLuma8(buffer) => {
            bytes8 = &**buffer;
            &bytes8[..]
        }
        DynamicImage::ImageLumaA8(buffer) => {
            bytes8 = &**buffer;
            &bytes8[..]
        }
        DynamicImage::ImageRgb8(buffer) => {
            bytes8 = &**buffer;
            &bytes8[..]
        }
        DynamicImage::ImageRgba8(buffer) => {
            bytes8 = &**buffer;
            &bytes8[..]
        }
        DynamicImage::ImageBgr8(buffer) => {
            bytes8 = &**buffer;
            &bytes8[..]
        }
        DynamicImage::ImageBgra8(buffer) => {
            bytes8 = &**buffer;
            &bytes8[..]
        }
        DynamicImage::ImageLuma16(buffer) => {
            bytes16 = &**buffer;
            bytemuck::cast_slice(&bytes16[..])
        }
        DynamicImage::ImageLumaA16(buffer) => {
            bytes16 = &**buffer;
            bytemuck::cast_slice(&bytes16[..])
        }
        DynamicImage::ImageRgb16(buffer) => {
            bytes16 = &**buffer;
            bytemuck::cast_slice(&bytes16[..])
        }
        DynamicImage::ImageRgba16(buffer) => {
            bytes16 = &**buffer;
            bytemuck::cast_slice(&bytes16[..])
        }
    };
    let image = graphics.create_image_static(
        ImageInfo {
            extent: ImageExtent::D2 {
                width: w,
                height: h,
            },
            format,
            levels: 1,
            layers: 1,
            samples: Samples1,
            usage: ImageUsage::SAMPLED,
        },
        Layout::ShaderReadOnlyOptimal,
        0,
        0,
        &bytes,
    )?;

    let view = graphics.create_image_view(ImageViewInfo::new(image))?;
    Ok(view)
}
