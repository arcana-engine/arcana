use std::{
    borrow::BorrowMut,
    convert::Infallible,
    fmt,
    future::{ready, Future, Ready},
    pin::Pin,
    task::{Context, Poll},
};

use goods::{
    Asset, AssetBuild, AssetField, AssetFieldBuild, AssetHandle, AssetId, AssetResult, Container,
    Loader,
};
use serde::ser::SerializeStruct;
use sierra::{
    CreateImageError, ImageExtent, ImageInfo, ImageUsage, ImageView, ImageViewInfo, Layout,
    Sampler, SamplerInfo, Samples::Samples1,
};

use crate::{graphics::Graphics, is_default};

use super::image::QoiImage;

pub fn texture_view_from_qoi_image(
    qoi: &rapid_qoi::Qoi,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Texture {
    /// Image view of the loaded texture.
    pub image: ImageView,

    /// Sampler associated with the texture image.
    pub sampler: Sampler,
}

pub struct TextureDecoded {
    texture: AssetResult<Texture>,
    sampler: SamplerInfo,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to load sub-asset")]
pub enum TextureAssetError {
    ImageLoadError {
        #[from]
        source: goods::Error,
    },
    SamplerCreateError(#[from] sierra::OutOfMemory),
}

#[derive(Clone, Copy, Debug)]
pub struct TextureInfo {
    pub image: AssetId,
    pub sampler: SamplerInfo,
}

impl TextureInfo {
    pub fn image(image: AssetId) -> Self {
        TextureInfo {
            image,
            sampler: SamplerInfo::default(),
        }
    }
}

impl serde::Serialize for TextureInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if is_default(&self.sampler) {
            self.image.serialize(serializer)
        } else {
            let mut serializer = serializer.serialize_struct("TextureInfo", 2)?;
            serializer.serialize_field("image", &self.image)?;
            serializer.serialize_field("sampler", &self.sampler)?;
            serializer.end()
        }
    }
}

impl<'de> serde::Deserialize<'de> for TextureInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::IntoDeserializer;
        struct Visitor;

        #[derive(serde::Deserialize)]
        struct ImageSamplerInfo {
            image: AssetId,
            sampler: SamplerInfo,
        }

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = TextureInfo;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Expecting struct or uuid")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(TextureInfo {
                    image: serde::Deserialize::deserialize(v.into_deserializer())?,
                    sampler: SamplerInfo::default(),
                })
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(TextureInfo {
                    image: serde::Deserialize::deserialize(v.into_deserializer())?,
                    sampler: SamplerInfo::default(),
                })
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let info: ImageSamplerInfo = serde::Deserialize::deserialize(
                    serde::de::value::SeqAccessDeserializer::new(seq),
                )?;
                Ok(TextureInfo {
                    image: info.image,
                    sampler: info.sampler,
                })
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let info: ImageSamplerInfo = serde::Deserialize::deserialize(
                    serde::de::value::MapAccessDeserializer::new(map),
                )?;
                Ok(TextureInfo {
                    image: info.image,
                    sampler: info.sampler,
                })
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(TextureInfo {
                    image: AssetId::new(v).ok_or_else(|| E::custom("AssetId cannot be zero"))?,
                    sampler: SamplerInfo::default(),
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

pub struct TextureFuture {
    image: AssetHandle<Texture>,
    sampler: SamplerInfo,
}

impl Future for TextureFuture {
    type Output = Result<TextureDecoded, Infallible>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<TextureDecoded, Infallible>> {
        let image = unsafe { self.as_mut().map_unchecked_mut(|me| &mut me.image) };

        match image.poll(cx) {
            Poll::Ready(image) => Poll::Ready(Ok(TextureDecoded {
                texture: image,
                sampler: self.sampler,
            })),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AssetField<Container> for Texture {
    type Info = TextureInfo;
    type DecodeError = Infallible;
    type BuildError = TextureAssetError;
    type Decoded = TextureDecoded;
    type Fut = TextureFuture;

    fn decode(info: TextureInfo, loader: &Loader) -> TextureFuture {
        TextureFuture {
            image: loader.load(info.image),
            sampler: info.sampler,
        }
    }
}

impl<B> AssetFieldBuild<Container, B> for Texture
where
    B: BorrowMut<Graphics>,
{
    fn build(mut decoded: TextureDecoded, builder: &mut B) -> Result<Self, TextureAssetError> {
        let graphics: &mut Graphics = builder.borrow_mut();
        let image = decoded.texture.build(graphics)?.image.clone();
        let sampler = graphics.create_sampler(decoded.sampler)?;
        Ok(Texture { image, sampler })
    }
}

impl Asset for Texture {
    type DecodeError = rapid_qoi::DecodeError;
    type BuildError = CreateImageError;
    type Decoded = QoiImage;
    type Fut = Ready<Result<QoiImage, rapid_qoi::DecodeError>>;

    fn name() -> &'static str {
        "qoi"
    }

    fn decode(bytes: Box<[u8]>, _loader: &Loader) -> Self::Fut {
        ready(
            rapid_qoi::Qoi::decode_alloc(&bytes).map(|(qoi, pixels)| QoiImage {
                qoi,
                pixels: pixels.into(),
            }),
        )
    }
}

impl<B> AssetBuild<B> for Texture
where
    B: BorrowMut<Graphics>,
{
    fn build(image: QoiImage, builder: &mut B) -> Result<Self, CreateImageError> {
        let graphics = builder.borrow_mut();
        let image = texture_view_from_qoi_image(&image.qoi, &image.pixels, graphics)?;

        Ok(Texture {
            image,
            sampler: graphics.create_sampler(Default::default())?,
        })
    }
}
