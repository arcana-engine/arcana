use goods::AssetField;
use hashbrown::HashMap;
use ordered_float::OrderedFloat;

#[cfg(feature = "physics2d")]
use parry2d::shape::SharedShape;

#[cfg(feature = "visible")]
use crate::graphics::Texture;
use crate::resources::Res;

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
#[cfg_attr(feature = "visible", derive(AssetField))]
#[cfg_attr(not(feature = "visible"), derive(serde::Deserialize))]
pub struct Tile {
    #[cfg(feature = "physics2d")]
    #[serde(default)]
    pub collider: Option<ColliderKind>,

    #[cfg(feature = "visible")]
    #[asset(container)]
    pub texture: Option<Texture>,
}
