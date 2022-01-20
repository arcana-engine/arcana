use std::{
    ops::Deref,
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;
use goods::{AssetBuild, AssetHandle, AssetId, AssetResult, DecodeError};
use pin_project::pin_project;
use serde_json::error::Category;

use {crate::clocks::TimeSpan, goods::Asset, std::sync::Arc};

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct SpriteSize {
    pub w: u32,
    pub h: u32,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct SpriteRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpriteFrame {
    pub tex: SpriteRect,
    pub src: SpriteRect,
    pub src_size: SpriteSize,
    pub span: TimeSpan,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct SpriteSheetMeta {
    pub frames: Arc<[SpriteFrame]>,

    #[serde(default = "default_distances")]
    pub frame_distances: Arc<[f32]>,

    #[serde(default = "default_animations")]
    pub animations: Arc<[SpriteAnimation]>,

    pub tex_size: SpriteSize,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct SpriteSheet<T> {
    pub meta: SpriteSheetMeta,
    pub texture: T,
}

#[pin_project(project = SSFP)]
pub enum SpriteSheetFut<T> {
    Ok {
        meta: Option<SpriteSheetMeta>,
        #[pin]
        texture: AssetHandle<T>,
    },
    Err(Option<DecodeError>),
}

impl<T> Future for SpriteSheetFut<T>
where
    T: Asset,
{
    type Output = Result<SpriteSheet<AssetResult<T>>, DecodeError>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<SpriteSheet<AssetResult<T>>, DecodeError>> {
        let me = self.project();
        match me {
            SSFP::Ok { meta, texture } => match texture.poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(texture) => Poll::Ready(Ok(SpriteSheet {
                    meta: meta.take().expect("SpriteSheetFut polled after it resoved"),
                    texture,
                })),
            },
            SSFP::Err(err) => Poll::Ready(Err(err
                .take()
                .expect("SpriteSheetFut polled after it resoved"))),
        }
    }
}

impl<T> Asset for SpriteSheet<T>
where
    T: Asset,
{
    type Decoded = SpriteSheet<AssetResult<T>>;
    type DecodeError = goods::DecodeError;
    type BuildError = goods::Error;
    type Fut = SpriteSheetFut<T>;

    fn name() -> &'static str {
        "arcana.spritesheet"
    }

    fn decode(bytes: Box<[u8]>, loader: &goods::Loader) -> SpriteSheetFut<T> {
        // Zero-length is definitely bincode.
        let info: SpriteSheet<AssetId> = if bytes.is_empty() {
            match ::goods::bincode::deserialize(&*bytes) {
                Ok(value) => value,
                Err(err) => return SpriteSheetFut::Err(Some(DecodeError::Bincode(err))),
            }
        } else {
            match ::goods::serde_json::from_slice(&*bytes) {
                Ok(value) => value,
                Err(err) => match err.classify() {
                    Category::Syntax => {
                        // That's not json. Bincode then.
                        match ::goods::bincode::deserialize(&*bytes) {
                            Ok(value) => value,
                            Err(err) => {
                                return SpriteSheetFut::Err(Some(DecodeError::Bincode(err)))
                            }
                        }
                    }
                    _ => return SpriteSheetFut::Err(Some(DecodeError::Json(err))),
                },
            }
        };

        let texture = loader.load(info.texture);

        SpriteSheetFut::Ok {
            meta: Some(info.meta),
            texture,
        }
    }
}

impl<B, T> AssetBuild<B> for SpriteSheet<T>
where
    T: AssetBuild<B>,
{
    fn build(
        mut decoded: SpriteSheet<AssetResult<T>>,
        builder: &mut B,
    ) -> Result<Self, goods::Error> {
        let texture = decoded.texture.build(builder)?;

        Ok(SpriteSheet {
            meta: decoded.meta,
            texture: texture.clone(),
        })
    }
}

impl<T> Deref for SpriteSheet<T> {
    type Target = SpriteSheetMeta;

    fn deref(&self) -> &SpriteSheetMeta {
        &self.meta
    }
}

fn default_distances() -> Arc<[f32]> {
    Arc::new([])
}

fn default_animations() -> Arc<[SpriteAnimation]> {
    Arc::new([])
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub frames: Vec<SpriteFrame>,
    pub animations: Vec<SpriteAnimation>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct SpriteAnimation {
    pub name: Box<str>,
    pub from: usize,
    pub to: usize,

    #[serde(default)]
    pub features: serde_json::Value,
}
