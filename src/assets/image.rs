use {
    super::{
        asset::Asset,
        format::{AssetDefaultFormat, Format},
        Loader,
    },
    crate::graphics::Graphics,
    image::{load_from_memory, DynamicImage, GenericImageView as _, ImageError},
    sierra::{
        CreateImageError, ImageExtent, ImageInfo, ImageUsage, ImageView, ImageViewInfo, Layout,
        Samples1,
    },
    std::future::{ready, Ready},
};

/// Image asset.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImageAsset {
    pub image: ImageView,
}

impl ImageAsset {
    pub fn into_inner(self) -> ImageView {
        self.image
    }
}

impl Asset for ImageAsset {
    type Error = CreateImageError;
    type Decoded = DynamicImage;
    type Builder = Graphics;

    fn build(image: DynamicImage, graphics: &mut Graphics) -> Result<Self, CreateImageError> {
        let image = image.to_rgba8();
        let image = image_view_from_dyn_image(&DynamicImage::ImageRgba8(image), graphics)?;

        Ok(ImageAsset { image })
    }
}

/// Quasi-format that tries to guess image format.
#[derive(Debug, Default)]
pub struct GuessImageFormat;

impl Format<ImageAsset> for GuessImageFormat {
    type Error = ImageError;

    type Fut = Ready<Result<DynamicImage, ImageError>>;
    fn decode(self, bytes: Box<[u8]>, _key: &str, _loader: Loader) -> Self::Fut {
        ready(load_from_memory(&bytes))
    }
}

impl AssetDefaultFormat for ImageAsset {
    type DefaultFormat = GuessImageFormat;
}

pub fn image_view_from_dyn_image(
    image: &DynamicImage,
    graphics: &mut Graphics,
) -> Result<ImageView, CreateImageError> {
    use sierra::Format;

    let format = match &image {
        DynamicImage::ImageLuma8(_) => Format::R8Unorm,
        DynamicImage::ImageLumaA8(_) => Format::RG8Unorm,
        DynamicImage::ImageRgb8(_) => Format::RGB8Unorm,
        DynamicImage::ImageRgba8(_) => Format::RGBA8Unorm,
        DynamicImage::ImageBgr8(_) => Format::BGR8Unorm,
        DynamicImage::ImageBgra8(_) => Format::BGRA8Unorm,
        DynamicImage::ImageLuma16(_) => Format::R16Unorm,
        DynamicImage::ImageLumaA16(_) => Format::RG16Unorm,
        DynamicImage::ImageRgb16(_) => Format::RGB16Unorm,
        DynamicImage::ImageRgba16(_) => Format::RGBA16Unorm,
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
