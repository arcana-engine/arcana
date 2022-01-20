cfg_if::cfg_if! {
    if #[cfg(feature = "graphics")] {
        use goods::AssetField;
        use crate::graphics::Texture;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "physics2d")] {
        use hashbrown::HashMap;
        use ordered_float::OrderedFloat;
        use parry2d::shape::SharedShape;
        use crate::resources::Res;
    }
}

#[cfg(feature = "physics2d")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[cfg_attr(feature = "server", derive(serde::Serialize))]
#[serde(rename_all = "snake_case")]
pub enum ColliderKind {
    Wall,
}

#[cfg(feature = "physics2d")]
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
#[cfg_attr(not(feature = "graphics"), derive(serde::Deserialize))]
pub struct Tile {
    #[cfg(feature = "physics2d")]
    #[serde(default)]
    pub collider: Option<ColliderKind>,

    #[cfg(feature = "graphics")]
    #[asset(container)]
    pub texture: Option<Texture>,
}
