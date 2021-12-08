use goods::Asset;
use hashbrown::HashMap;

use crate::graphics::{Texture, UV};

#[derive(Clone, Debug, Asset)]
#[asset(name = "arcana.font")]
pub struct Font {
    #[asset(external)]
    msdf: ImageAsset,
    glyph_uv: HashMap<u16, UV>,
}

impl Font {
    pub fn texture(&self) -> &ImageView {
        self.msdf.get_ref()
    }
}
