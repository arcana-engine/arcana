cfg_if::cfg_if! {
    if #[cfg(feature = "graphics")] {
        use arcana::{
            assets::AssetField,
            rect::Rect,
            graphics::Texture,
        };
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "physics")] {
        use hashbrown::HashMap;
        use ordered_float::OrderedFloat;
        use arcana_physics::physics2::shape::SharedShape;
        use arcana::resources::Res;
    }
}

#[cfg(feature = "physics")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColliderKind {
    Wall,
}

#[cfg(feature = "physics")]
impl ColliderKind {
    pub fn shared_shape(&self, size: f32, res: &mut Res) -> SharedShape {
        struct TileShapes(HashMap<(ColliderKind, OrderedFloat<f32>), SharedShape>);
        let shapes = res.with(|| TileShapes(HashMap::new()));

        match shapes.0.get(&(*self, OrderedFloat(size))) {
            Some(shape) => shape.clone(),
            None => {
                let shape = SharedShape::cuboid(size * 0.5, size * 0.5);
                shapes.0.insert((*self, OrderedFloat(size)), shape.clone());
                shape
            }
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "graphics", derive(AssetField))]
#[cfg_attr(
    not(feature = "graphics"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Tile {
    #[cfg(feature = "physics")]
    #[serde(default)]
    pub collider: Option<ColliderKind>,

    #[cfg(feature = "graphics")]
    #[asset(container)]
    pub texture: Option<Texture>,

    #[cfg(feature = "graphics")]
    #[serde(default)]
    pub uv: Rect,
}
