use {
    crate::graphics::{Rect, Texture},
    goods::Asset,
    std::sync::Arc,
};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct SpriteFrame {
    pub src: Rect,
    pub dst: Rect,

    #[serde(default)]
    pub duration_us: u64,
}

#[derive(Clone, Debug, Asset)]
pub struct SpriteSheet {
    pub frames: Arc<[SpriteFrame]>,

    #[serde(default = "default_distances")]
    pub frame_distances: Arc<[f32]>,

    #[serde(default = "default_animations")]
    pub animations: Arc<[SpriteAnimation]>,

    #[container]
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

#[derive(Clone, Debug, serde::Deserialize)]
pub struct SpriteAnimation {
    pub name: Box<str>,
    pub from: usize,
    pub to: usize,

    #[serde(default)]
    pub features: serde_json::Value,
}
