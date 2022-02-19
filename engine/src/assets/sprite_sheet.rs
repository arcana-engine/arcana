use std::sync::Arc;

use arcana_time::TimeSpan;
use goods::Asset;

use super::texture::Texture;

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

#[derive(Clone, Debug, Asset)]
#[asset(name = "arcana.spritesheet")]
pub struct SpriteSheet {
    pub frames: Arc<[SpriteFrame]>,

    #[serde(default = "default_distances")]
    pub frame_distances: Arc<[f32]>,

    #[serde(default = "default_animations")]
    pub animations: Arc<[SpriteAnimation]>,

    #[serde(rename = "tex-size")]
    pub tex_size: SpriteSize,

    #[asset(container)]
    pub texture: Texture,
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SpriteAnimation {
    pub name: Box<str>,
    pub from: usize,
    pub to: usize,

    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub features: serde_json::Value,
}
