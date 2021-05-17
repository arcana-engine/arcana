pub use {
    super::Graphics,
    crate::assets::ImageAsset,
    goods::{
        Asset, AssetBuild, AssetField, AssetFieldBuild, AssetHandle, AssetResult, Container, Error,
        Loader,
    },
    sierra::{ImageView, OutOfMemory, Sampler, SamplerInfo},
    std::{
        borrow::BorrowMut,
        convert::Infallible,
        fmt,
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
    uuid::Uuid,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Texture {
    /// Image view of the loaded texture.
    pub image: ImageView,

    /// Sampler associated with the texture image.
    pub sampler: Sampler,
}

pub struct TextureDecoded {
    image: AssetResult<ImageAsset>,
    sampler: SamplerInfo,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to load sub-asset")]
pub enum TextureAssetError {
    ImageLoadError {
        #[from]
        source: Error,
    },
    SamplerCreateError(#[from] OutOfMemory),
}

pub struct TextureInfo {
    image: Uuid,
    sampler: SamplerInfo,
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
            image: Uuid,
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

            fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(TextureInfo {
                    image: Uuid::from_u128(v),
                    sampler: SamplerInfo::default(),
                })
            }
        }

        deserializer.deserialize_struct("TextureInfo", &["image", "sampler"], Visitor)
    }
}

pub struct TextureFuture {
    image: AssetHandle<ImageAsset>,
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
                image,
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
            image: loader.load(&info.image),
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
        let image = decoded.image.get(graphics)?.0.clone();
        let sampler = graphics.create_sampler(decoded.sampler)?;
        Ok(Texture { image, sampler })
    }
}
