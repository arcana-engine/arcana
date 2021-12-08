use sierra::Image;

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

impl ImageAsset {
    pub fn get_ref(&self) -> &ImageView {
        &self.0
    }

    pub fn get_mut(&mut self) -> &mut ImageView {
        &mut self.0
    }
}

impl From<ImageAsset> for ImageView {
    fn from(asset: ImageAsset) -> Self {
        asset.0
    }
}

pub struct QoiImage {
    pub qoi: rapid_qoi::Qoi,
    pub pixels: Vec<u8>,
}

impl Asset for ImageAsset {
    type DecodeError = rapid_qoi::DecodeError;
    type BuildError = CreateImageError;
    type Decoded = QoiImage;
    type Fut = Ready<Result<QoiImage, rapid_qoi::DecodeError>>;

    fn decode(bytes: Box<[u8]>, _loader: &Loader) -> Self::Fut {
        ready(rapid_qoi::Qoi::decode_alloc(&bytes).map(|(qoi, pixels)| QoiImage { qoi, pixels }))
    }
}

impl<B> AssetBuild<B> for ImageAsset
where
    B: BorrowMut<Graphics>,
{
    fn build(image: PngImage, builder: &mut B) -> Result<Self, CreateImageError> {
        let image = sampled_image_from_qoi_image(&image.qoi, &image.pixels, builder.borrow_mut())?;

        Ok(ImageAsset(image))
    }
}

pub fn texture_view_from_qoi_image(
    qoi: &Qoi,
    pixels: &[u8],
    graphics: &mut Graphics,
) -> Result<ImageView, CreateImageError> {
    use sierra::Format;

    let image = graphics.create_image_static(
        ImageInfo {
            extent: ImageExtent::D2 {
                width: qoi.width,
                height: qoi.height,
            },
            format: Format::RGBA8Srgb,
            levels: 1,
            layers: 1,
            samples: Samples1,
            usage: ImageUsage::SAMPLED,
        },
        Layout::ShaderReadOnlyOptimal,
        0,
        0,
        pixels,
    )?;

    let view = graphics.create_image_view(ImageViewInfo::new(image))?;
    Ok(view)
}
