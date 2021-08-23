use {
    crate::graphics::Graphics,
    goods::{Asset, AssetBuild, Loader},
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

pub struct PngImage {
    decoded: Box<[u8]>,
    width: u32,
    height: u32,
    bit_depth: png::BitDepth,
    color_type: png::ColorType,
    srgb: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ImageDecodeError {
    #[error(transparent)]
    Png(#[from] png::DecodingError),

    #[error("Unsupported color type {0:?}")]
    UnsupportedColorType(png::ColorType),

    #[error("Unsupported bit depth {0:?}")]
    UnsupportedBitDepth(png::BitDepth),

    #[error("Unsupported bit depth {0:?}")]
    UnsupportedSrgbBitDepth(png::BitDepth),
}

impl Asset for ImageAsset {
    type DecodeError = ImageDecodeError;
    type BuildError = CreateImageError;
    type Decoded = PngImage;
    type Fut = Ready<Result<PngImage, ImageDecodeError>>;

    fn decode(bytes: Box<[u8]>, _loader: &Loader) -> Self::Fut {
        let mut decoder = png::Decoder::new(&*bytes);
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = match decoder.read_info() {
            Err(err) => return ready(Err(err.into())),
            Ok(reader) => reader,
        };

        let info = reader.info();
        let srgb = info.srgb.is_some();

        match info.bit_depth {
            png::BitDepth::Eight => {}
            png::BitDepth::Sixteen => {
                if srgb {
                    return ready(Err(ImageDecodeError::UnsupportedSrgbBitDepth(
                        info.bit_depth,
                    )));
                }
            }
            _ => return ready(Err(ImageDecodeError::UnsupportedBitDepth(info.bit_depth))),
        }
        match info.color_type {
            png::ColorType::Grayscale
            | png::ColorType::GrayscaleAlpha
            | png::ColorType::Rgb
            | png::ColorType::Rgba => {}
            _ => return ready(Err(ImageDecodeError::UnsupportedBitDepth(info.bit_depth))),
        }

        let mut buf = vec![0; reader.output_buffer_size()];
        let info = match reader.next_frame(&mut buf) {
            Err(err) => return ready(Err(err.into())),
            Ok(info) => info,
        };
        buf.truncate(info.buffer_size());

        let image = PngImage {
            decoded: buf.into_boxed_slice(),
            width: info.width,
            height: info.height,
            bit_depth: info.bit_depth,
            color_type: info.color_type,
            srgb,
        };

        ready(Ok(image))
    }
}

impl<B> AssetBuild<B> for ImageAsset
where
    B: BorrowMut<Graphics>,
{
    fn build(image: PngImage, builder: &mut B) -> Result<Self, CreateImageError> {
        let image = image_view_from_png_image(&image, builder.borrow_mut())?;

        Ok(ImageAsset(image))
    }
}

pub fn image_view_from_png_image(
    image: &PngImage,
    graphics: &mut Graphics,
) -> Result<ImageView, CreateImageError> {
    use sierra::Format;

    let format = match (image.srgb, image.color_type, image.bit_depth) {
        (true, png::ColorType::Grayscale, png::BitDepth::Eight) => Format::R8Srgb,
        (true, png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => Format::RG8Srgb,
        (true, png::ColorType::Rgb, png::BitDepth::Eight) => Format::RGB8Srgb,
        (true, png::ColorType::Rgba, png::BitDepth::Eight) => Format::RGBA8Srgb,
        (false, png::ColorType::Grayscale, png::BitDepth::Eight) => Format::R8Unorm,
        (false, png::ColorType::Grayscale, png::BitDepth::Sixteen) => Format::R16Unorm,
        (false, png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => Format::RG8Unorm,
        (false, png::ColorType::GrayscaleAlpha, png::BitDepth::Sixteen) => Format::RG16Unorm,
        (false, png::ColorType::Rgb, png::BitDepth::Eight) => Format::RGB8Unorm,
        (false, png::ColorType::Rgb, png::BitDepth::Sixteen) => Format::RGB16Unorm,
        (false, png::ColorType::Rgba, png::BitDepth::Eight) => Format::RGBA8Unorm,
        (false, png::ColorType::Rgba, png::BitDepth::Sixteen) => Format::RGBA16Unorm,
        _ => panic!("Unsupported format"),
    };

    let image = graphics.create_image_static(
        ImageInfo {
            extent: ImageExtent::D2 {
                width: image.width,
                height: image.height,
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
        &*image.decoded,
    )?;

    let view = graphics.create_image_view(ImageViewInfo::new(image))?;
    Ok(view)
}
