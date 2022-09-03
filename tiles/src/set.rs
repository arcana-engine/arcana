use std::sync::Arc;

use goods::Asset;

use super::tile::Tile;

#[derive(Clone, Debug, Asset)]
#[asset(name = "arcana.tile-set")]
pub struct TileSet {
    #[cfg_attr(feature = "graphics", asset(container))]
    pub tiles: Arc<[Tile]>,
}
