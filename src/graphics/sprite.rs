#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Default for Rect {
    fn default() -> Self {
        Rect {
            left: 0.0,
            right: 1.0,
            top: 0.0,
            bottom: 1.0,
        }
    }
}

/// Sprite component.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Sprite {
    /// Rect of the sprite to render.
    pub pos: Rect,

    /// Rect of sprite texture portion.
    pub uv: Rect,

    /// Layer at which sprite should be rendered
    /// Layers are relative, the higher level sprites are rendered over
    /// lower layer sprites.
    pub layer: u32,
}

mod serde_impls {
    use {super::*, serde::de::*};

    #[derive(serde::Deserialize)]
    struct LRTB {
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
    }

    #[derive(serde::Deserialize)]
    struct XYWH {
        x: f32,
        y: f32,

        #[serde(alias = "width")]
        w: f32,

        #[serde(alias = "height")]
        h: f32,
    }

    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum AnyRect {
        LRTB(LRTB),
        XYWH(XYWH),
    }

    impl<'de> Deserialize<'de> for Rect {
        fn deserialize<D>(deserializer: D) -> Result<Rect, D::Error>
        where
            D: Deserializer<'de>,
        {
            let rect = match AnyRect::deserialize(deserializer)? {
                AnyRect::LRTB(lrtb) => Rect {
                    left: lrtb.left,
                    right: lrtb.right,
                    top: lrtb.top,
                    bottom: lrtb.bottom,
                },
                AnyRect::XYWH(xywh) => Rect {
                    left: xywh.x,
                    right: xywh.x + xywh.w,
                    top: xywh.y,
                    bottom: xywh.y + xywh.h,
                },
            };
            Ok(rect)
        }
    }
}
