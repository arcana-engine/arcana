use std::{path::Path, sync::Arc};

use arcana_time::TimeSpan;
use treasury_import::{Dependencies, Dependency, ImportError, Importer, Sources};

use crate::{
    graphics::TextureInfo,
    sprite::{SpriteAnimation, SpriteFrame, SpriteRect, SpriteSheetInfo, SpriteSize},
};

pub struct SpriteSheetImporter;

#[derive(serde::Deserialize)]
struct Frame {
    /// Rect on sprite-sheet.
    frame: SpriteRect,

    /// Corresponding rect on source image.
    #[serde(rename = "spriteSourceSize")]
    sprite_source_size: SpriteRect,

    /// Corresponding rect on source image.
    #[serde(rename = "sourceSize")]
    source_size: SpriteSize,

    /// Frame duration in milliseconds.
    #[serde(rename = "duration")]
    duration_ms: u64,
}

#[derive(serde::Deserialize)]
enum Format {
    RGBA8888,
}

#[derive(serde::Deserialize)]
struct FrameTag {
    name: String,
    from: usize,
    to: usize,
}

#[derive(serde::Deserialize)]
struct SpriteSheetMeta {
    image: String,
    size: SpriteSize,

    #[serde(rename = "frameTags", default)]
    frame_tags: Vec<FrameTag>,
}

#[derive(serde::Deserialize)]
struct AsepriteSpriteSheet {
    frames: Vec<Frame>,
    meta: SpriteSheetMeta,
}

impl Importer for SpriteSheetImporter {
    fn name(&self) -> &str {
        "Aseprite spritesheet"
    }

    fn formats(&self) -> &[&str] {
        &["aseprite.spritesheet"]
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }

    fn target(&self) -> &str {
        "arcana.spritesheet"
    }

    fn import(
        &self,
        source_path: &Path,
        native_path: &Path,
        _sources: &mut (impl Sources + ?Sized),
        dependencies: &mut (impl Dependencies + ?Sized),
    ) -> Result<(), ImportError> {
        let source = std::fs::read(source_path).map_err(|err| ImportError::Other {
            reason: format!(
                "Failed to open file: '{}'. {:#}",
                source_path.display(),
                err
            ),
        })?;

        let sprite_sheet: AsepriteSpriteSheet =
            serde_json::from_slice(&source).map_err(|err| ImportError::Other {
                reason: format!(
                    "Failed to parse file: '{}' as AsepriteSpriteSheet. {:#}",
                    source_path.display(),
                    err,
                ),
            })?;

        let image = match dependencies.get(&sprite_sheet.meta.image, "qoi") {
            Err(err) => {
                return Err(ImportError::Other {
                    reason: format!("Failed to fetch image of the spritesheet. {:#}", err),
                })
            }
            Ok(None) => {
                return Err(ImportError::RequireDependencies {
                    dependencies: vec![Dependency {
                        source: sprite_sheet.meta.image.clone(),
                        target: "qoi".to_owned(),
                    }],
                })
            }
            Ok(Some(id)) => id,
        };

        let frames = sprite_sheet
            .frames
            .iter()
            .enumerate()
            .map(|(index, frame)| {
                if frame.frame.w != frame.sprite_source_size.w {
                    return Err(ImportError::Other {
                        reason: format!("Frame '{}' width does not match source", index),
                    });
                }

                if frame.frame.h != frame.sprite_source_size.h {
                    return Err(ImportError::Other {
                        reason: format!("Frame '{}' height does not match source", index),
                    });
                }

                Ok(SpriteFrame {
                    tex: frame.frame,
                    src: frame.sprite_source_size,
                    src_size: frame.source_size,
                    span: frame.duration_ms * TimeSpan::MILLISECOND,
                })
            })
            .collect::<Result<_, _>>()?;

        let animations = sprite_sheet
            .meta
            .frame_tags
            .into_iter()
            .map(|tag| SpriteAnimation {
                name: tag.name.into(),
                from: tag.from,
                to: tag.to,
                features: serde_json::Value::Null,
            })
            .collect();

        let sprite_sheet = SpriteSheetInfo {
            tex_size: sprite_sheet.meta.size,
            frames,
            animations,
            texture: TextureInfo::image(goods::AssetId(image.value())),
            frame_distances: Arc::new([]),
        };

        let mut output = std::fs::File::create(native_path).map_err(|err| ImportError::Other {
            reason: format!(
                "Failed to open file: '{}'. {:#}",
                native_path.display(),
                err
            ),
        })?;

        serde_json::to_writer(&mut output, &sprite_sheet).map_err(|err| ImportError::Other {
            reason: format!(
                "Failed to write SpriteSheet into file: '{}'. {:#}",
                native_path.display(),
                err,
            ),
        })?;

        Ok(())
    }
}
