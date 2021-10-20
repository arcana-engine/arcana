use std::sync::Arc;

use goods::Asset;

use super::tile::Tile;

#[derive(Clone, Debug, Asset)]
pub struct TileSet {
    #[cfg_attr(feature = "visible", container)]
    pub tiles: Arc<[Tile]>,
}
