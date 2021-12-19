use fontdue::Font;
use goods::{Asset, AssetId, TrivialAsset};
use hashbrown::hash_map::{Entry, HashMap};
use sierra::ImageView;

use crate::rect::Rect;

use super::ImageAsset;

#[derive(Clone, Debug)]
pub struct FontAsset {
    inner: Font,
}

#[derive(Clone, Copy, Debug, thiserror::Error)]
#[error("Failed to parse font. {description}")]
pub struct FontParseError {
    description: &'static str,
}

impl TrivialAsset for FontAsset {
    type Error = FontParseError;

    fn name() -> &'static str {
        "arcana.font"
    }

    fn decode(bytes: Box<[u8]>) -> Result<Self, FontParseError> {
        match Font::from_bytes(&*bytes, Default::default()) {
            Ok(font) => Ok(FontAsset { inner: font }),
            Err(err) => Err(FontParseError { description: err }),
        }
    }
}

#[derive(Clone, Debug, Asset)]
#[asset(name = "arcana.font-faces")]
pub struct FontFaces {
    #[asset(external)]
    font: FontAsset,
    #[asset(external)]
    msdf: ImageAsset,
    glyphs_uv: HashMap<u16, Rect>,
}

impl FontFaces {
    pub fn font(&self) -> &Font {
        &self.font.inner
    }

    pub fn texture(&self) -> &ImageView {
        self.msdf.get_ref()
    }

    pub fn glyph_uv(&self, glyph: u16) -> Option<Rect> {
        self.glyphs_uv.get(&glyph).copied()
    }
}

pub struct FontFacesCache {
    ids: HashMap<AssetId, (usize, usize)>,
    fonts: Vec<fontdue::Font>,
    msdfs: Vec<ImageView>,
    glyphs_uv: HashMap<(usize, u16), (usize, Rect)>,
}

impl FontFacesCache {
    pub fn new() -> Self {
        FontFacesCache {
            ids: HashMap::new(),
            fonts: Vec::new(),
            msdfs: Vec::new(),
            glyphs_uv: HashMap::new(),
        }
    }

    pub fn add_font(&mut self, id: AssetId, faces: &FontFaces) -> usize {
        match self.ids.entry(id) {
            Entry::Occupied(entry) => {
                let (font_idx, msdf_idx) = *entry.get();

                let msdf = &self.msdfs[msdf_idx];
                if msdf != faces.texture() {
                    tracing::warn!(
                        "Attempt to add duplicate font '{}' with different texture",
                        id
                    );
                }

                for (glyph, uv) in &faces.glyphs_uv {
                    match self.glyphs_uv.get(&(font_idx, *glyph)) {
                        None => {
                            tracing::warn!("Missing glyph '{}' in cached font '{}'", glyph, id);
                        }
                        Some((idx, cached_uv)) => {
                            assert_eq!(*idx, msdf_idx);
                            if uv != cached_uv {
                                tracing::warn!("Glyph '{}' in cached font '{}' has UV '{}', readded with UV '{}'", glyph, id, cached_uv, uv);
                            }
                        }
                    }
                }

                font_idx
            }
            Entry::Vacant(mut entry) => {
                let font_idx = self.fonts.len();

                let msdf_idx = match self.msdfs.iter().position(|msdf| msdf == faces.texture()) {
                    None => {
                        let msdf_idx = self.msdfs.len();
                        self.msdfs.push(faces.texture().clone());
                        msdf_idx
                    }
                    Some(msdf_idx) => msdf_idx,
                };

                for (glyph, uv) in &faces.glyphs_uv {
                    self.glyphs_uv.insert((font_idx, *glyph), (msdf_idx, *uv));
                }

                entry.insert((font_idx, msdf_idx));
                font_idx
            }
        }
    }
}
