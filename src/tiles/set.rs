use std::sync::Arc;

use goods::Asset;

use super::tile::Tile;

#[derive(Clone, Debug, Asset)]
#[asset(name = "arcana.tile-set")]
pub struct TileSet {
    #[cfg_attr(feature = "visible", asset(container))]
    pub tiles: Arc<[Tile]>,
}
