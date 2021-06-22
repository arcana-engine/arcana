use {
    crate::sprite_sheet::{SpriteAnimation, SpriteFrame, SpriteRect, SpriteSheet, SpriteSize},
    arcana_timespan::TimeSpan,
    eyre::WrapErr,
    std::path::Path,
    treasury_import::{Importer, Registry},
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

    #[serde(rename = "frameTags")]
    frame_tags: Vec<FrameTag>,
}

#[derive(serde::Deserialize)]
struct AsepriteSpriteSheet {
    frames: Vec<Frame>,
    meta: SpriteSheetMeta,
}

impl Importer for SpriteSheetImporter {
    fn name(&self) -> &str {
        "arcana.aseprite.spritesheet"
    }

    fn source(&self) -> &str {
        "aseprite.spritesheet"
    }

    fn native(&self) -> &str {
        "arcana.spritesheet"
    }

    fn import(
        &self,
        source_path: &Path,
        native_path: &Path,
        registry: &mut dyn Registry,
    ) -> eyre::Result<()> {
        let source = std::fs::read(source_path)
            .wrap_err_with(|| format!("Failed to open file: '{}'", source_path.display()))?;

        let sprite_sheet: AsepriteSpriteSheet =
            serde_json::from_slice(&source).wrap_err_with(|| {
                format!(
                    "Failed to parse file: '{}' as AsepriteSpriteSheet",
                    source_path.display(),
                )
            })?;

        let frames = sprite_sheet
            .frames
            .iter()
            .enumerate()
            .map(|(index, frame)| {
                if frame.frame.w != frame.sprite_source_size.w {
                    return Err(eyre::eyre!("Frame '{}' width does not match source", index));
                }

                if frame.frame.h != frame.sprite_source_size.h {
                    return Err(eyre::eyre!(
                        "Frame '{}' height does not match source",
                        index
                    ));
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

        let texture = registry.store(
            &source_path.with_file_name(&sprite_sheet.meta.image),
            "image",
            "rgba.png",
            &["texture"],
        )?;

        let sprite_sheet = SpriteSheet {
            tex_size: sprite_sheet.meta.size,
            frames,
            animations,
            texture,
            frame_distances: Vec::new(),
        };

        let mut output = std::fs::File::create(native_path)
            .wrap_err_with(|| format!("Failed to open file: '{}'", native_path.display()))?;

        serde_json::to_writer(&mut output, &sprite_sheet).wrap_err_with(|| {
            format!(
                "Failed to write SpriteSheet into file: '{}'",
                native_path.display(),
            )
        })?;

        Ok(())
    }
}
