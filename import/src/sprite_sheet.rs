use {
    arcana_time::TimeSpan,
    eyre::WrapErr,
    goods_treasury_import::{Importer, Registry},
    image::GenericImageView,
    std::{convert::TryFrom as _, path::Path},
    uuid::Uuid,
};

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SpriteSize {
    pub w: u32,
    pub h: u32,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SpriteRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpriteFrame {
    pub tex: SpriteRect,
    pub src: SpriteRect,
    pub src_size: SpriteSize,
    pub span: TimeSpan,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SpriteAnimation {
    pub name: Box<str>,
    pub from: usize,
    pub to: usize,

    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub features: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpriteSheet {
    pub tex_size: SpriteSize,
    pub frames: Vec<SpriteFrame>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub frame_distances: Vec<f32>,
    pub animations: Vec<SpriteAnimation>,
    pub texture: Uuid,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SpriteSheetEnrichInfo {
    /// Source format for spritesheet
    source_format: String,

    /// Relative path to spritesheet
    main: String,

    /// Relative path to spritesheet
    posture: String,

    #[serde(default)]
    features: Vec<serde_json::Value>,
}

pub struct SpriteSheetEnrich;

impl Importer for SpriteSheetEnrich {
    fn name(&self) -> &str {
        "arcana.spritesheet.enrich"
    }

    fn source(&self) -> &str {
        "arcana.spritesheet.enrich"
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
        let file = std::fs::File::open(source_path)
            .wrap_err_with(|| format!("Failed to open file: '{}'", source_path.display()))?;

        let info: SpriteSheetEnrichInfo = serde_json::from_reader(file).wrap_err_with(|| {
            format!(
                "Failed to parse file: '{}' as SpriteSheetEnrichInfo",
                source_path.display()
            )
        })?;

        let main = registry.store(
            &source_path.with_file_name(&info.main),
            &info.source_format,
            "arcana.spritesheet",
            &["spritesheet"],
        )?;

        let main_path = registry.fetch(&main)?;
        let main_file = std::fs::File::open(&main_path)
            .wrap_err_with(|| format!("Failed to open file: '{}'", main_path.display()))?;
        let mut sprite_sheet: SpriteSheet =
            serde_json::from_reader(main_file).wrap_err_with(|| {
                format!(
                    "Failed to parse file: '{}' as SpriteSheet",
                    main_path.display()
                )
            })?;

        sprite_sheet.frame_distances.clear();
        for (animation, features) in sprite_sheet.animations.iter_mut().zip(&info.features) {
            animation.features = features.clone();
        }

        let posture = registry.store(
            &source_path.with_file_name(&info.posture),
            &info.source_format,
            "arcana.spritesheet",
            &["spritesheet", "posture"],
        )?;

        let posture_path = registry.fetch(&posture)?;

        let posture_file = std::fs::File::open(&posture_path)
            .wrap_err_with(|| format!("Failed to open file: '{}'", posture_path.display()))?;

        let posture_sheet: SpriteSheet =
            serde_json::from_reader(posture_file).wrap_err_with(|| {
                format!(
                    "Failed to parse file: '{}' as SpriteSheet",
                    posture_path.display()
                )
            })?;

        let posture_texture_path = registry.fetch(&posture_sheet.texture)?;

        let posture_texture_file =
            std::fs::File::open(&posture_texture_path).wrap_err_with(|| {
                format!(
                    "Failed to open image file '{}'",
                    posture_texture_path.display()
                )
            })?;

        let reader = std::io::BufReader::new(posture_texture_file);

        let posture_texture = image::load(reader, image::ImageFormat::Png).wrap_err_with(|| {
            format!(
                "Failed to open image file: '{}'",
                posture_texture_path.display()
            )
        })?;

        let (w, h) = posture_texture.dimensions();

        for f in &posture_sheet.frames {
            if f.tex.x + f.tex.w > w && f.tex.y + f.tex.h > h {
                Err(eyre::eyre!(
                    "Texture rect is out of bounds. Texture dimensions is {}x{}. Frame {{ x: {}, y: {}, w: {}, h: {} }}",
                    w, h, f.tex.x, f.tex.y, f.tex.w, f.tex.h,
                ))?;
            }

            if f.src.x + f.tex.w > f.src.w && f.src.y + f.tex.h > f.src.h {
                Err(eyre::eyre!(
                    "Source rect is out of bounds. Source dimensions is {}x{}. Frame {{ x: {}, y: {}, w: {}, h: {} }}",
                    f.src.w, f.src.h, f.src.x, f.tex.y, f.src.w, f.tex.h,
                ))?;
            }
        }

        if usize::try_from(w * h * 32).is_err() {
            Err(eyre::eyre!("posture texture is too large"))?;
        }

        let channel_count = posture_texture
            .pixels()
            .map(|(_, _, image::Rgba(rgba))| u32::from_ne_bytes(rgba).leading_zeros())
            .min()
            .map(|value| 32 - value)
            .unwrap_or(0);

        let mut sdfs = Vec::new();

        for frame in &posture_sheet.frames {
            let fx = frame.tex.x;
            let fy = frame.tex.y;
            let fw = frame.tex.w;
            let fh = frame.tex.h;

            let sx = frame.src.x;
            let sy = frame.src.y;
            let sw = frame.src.w;
            let sh = frame.src.h;

            let size = (channel_count * sw * sh) as usize;

            let mut sdf: Vec<_> = (0..size).map(|_| sw.max(sh)).collect();

            let mut queue = Vec::with_capacity(size);

            // Horizontal pass

            for y in 0..fh {
                for x in 0..fw {
                    if x + fx >= w || y + fy >= h {
                        panic!("Point is out of texture bounds");
                    }

                    let image::Rgba(rgba) = posture_texture.get_pixel(x + fx, y + fy);
                    let bits = u32::from_ne_bytes(rgba);

                    for bit in iter_bits(bits) {
                        assert!(
                            queue.capacity() > queue.len(),
                            "Queue capacity must not be exceeded\n\tcap: `{}`, len: `{}`\n\tx: `{}`, y: `{}`",
                            queue.capacity(),
                            queue.len(),
                            x, y,
                        );
                        queue.push((bit, x + sx, 0));
                    }
                }

                while let Some((bit, x, d)) = queue.pop() {
                    let index = (bit * sw * sh + (y + sy) * sw + x) as usize;

                    if index >= size {
                        panic!("Index out of bounds at bit {}, x: {}, y: {}", bit, x, y);
                    }

                    if d < sdf[index] {
                        sdf[index] = d;
                        if x > 0 {
                            assert!(
                            queue.capacity() > queue.len(),
                            "Queue capacity must not be exceeded\n\tcap: `{}`, len: `{}`\n\tlimit: `{}`",
                            queue.capacity(),
                            queue.len(),
                            size,
                        );
                            queue.push((bit, x - 1, d + 1));
                        }
                        if x + 1 < sw {
                            assert!(
                            queue.capacity() > queue.len(),
                            "Queue capacity must not be exceeded\n\tcap: `{}`, len: `{}`\n\tlimit: `{}`",
                            queue.capacity(),
                            queue.len(),
                            size,
                        );
                            queue.push((bit, x + 1, d + 1));
                        }
                    }
                }
            }

            // Vertical pass

            for x in 0..sw {
                for y in 0..sh {
                    for bit in 0..channel_count {
                        let index = (bit * sw * sh + y * sw + x) as usize;
                        if sdf[index] < u32::MAX {
                            if y > 0 {
                                queue.push((bit, y - 1, sdf[index] + 1));
                            }
                            if y + 1 < sh {
                                queue.push((bit, y + 1, sdf[index] + 1));
                            }
                        }
                    }
                }

                while let Some((bit, y, d)) = queue.pop() {
                    let index = (bit * sw * sh + y * sw + x) as usize;

                    if index >= size {
                        panic!("Index out of bounds at bit {}, x: {}, y: {}", bit, x, y);
                    }

                    if d < sdf[index] {
                        sdf[index] = d;
                        if y > 0 {
                            queue.push((bit, y - 1, d + 1));
                        }
                        if y + 1 < sh {
                            queue.push((bit, y + 1, d + 1));
                        }
                    }
                }
            }

            sdfs.push(sdf);
        }

        let sample_sdf =
            |sdf: &[u32], bit: u32, sx: u32, sw: u32, dw: u32, sy: u32, sh: u32, dh: u32| {
                let dy = (dh - 1).min((((sy as f32 + 0.5) / sh as f32) * dh as f32) as u32);
                let dx = (dw - 1).min((((sx as f32 + 0.5) / sw as f32) * dw as f32) as u32);

                let index = bit * dw * dh + dy * dw + dx;
                sdf[index as usize]
            };

        for i in 0..posture_sheet.frames.len() {
            for j in i + 1..posture_sheet.frames.len() {
                let left = &posture_sheet.frames[i];
                let right = &posture_sheet.frames[j];

                let left_sdf = &sdfs[i];
                let right_sdf = &sdfs[j];

                let mut left_acc = 0;
                let mut left_distance_acc = 0;

                for x in 0..left.tex.w {
                    for y in 0..left.tex.h {
                        let image::Rgba(rgba) =
                            posture_texture.get_pixel(x + left.tex.x, y + left.tex.y);
                        let bits = u32::from_ne_bytes(rgba);

                        for bit in iter_bits(bits) {
                            let d = sample_sdf(
                                right_sdf,
                                bit,
                                x + left.src.x,
                                left.src.w,
                                right.src.w,
                                y + left.src.y,
                                left.src.h,
                                right.src.h,
                            );

                            left_acc += 1u32;
                            left_distance_acc += d as u64;
                        }
                    }
                }

                let left_distance = left_distance_acc as f64 / left_acc as f64;

                let mut right_acc = 0;
                let mut right_distance_acc = 0;

                for x in 0..right.tex.w {
                    for y in 0..right.tex.h {
                        let image::Rgba(rgba) =
                            posture_texture.get_pixel(x + right.tex.x, y + right.tex.y);
                        let bits = u32::from_ne_bytes(rgba);

                        for bit in iter_bits(bits) {
                            let d = sample_sdf(
                                left_sdf,
                                bit,
                                x + right.src.x,
                                right.src.w,
                                left.src.w,
                                y + left.src.x,
                                right.src.h,
                                left.src.h,
                            );

                            right_acc += 1u32;
                            right_distance_acc += d as u64;
                        }
                    }
                }

                let right_distance = right_distance_acc as f64 / right_acc as f64;
                let distance = left_distance.max(right_distance);

                sprite_sheet.frame_distances.push(distance as f32);
            }
        }

        let native_file = std::fs::File::create(native_path)
            .wrap_err_with(|| format!("Failed to open file: '{}'", native_path.display()))?;
        serde_json::to_writer(native_file, &sprite_sheet).wrap_err_with(|| {
            format!(
                "Failed to write SpriteSheet into file: '{}'",
                native_path.display()
            )
        })?;

        Ok(())
    }
}

fn iter_bits(value: u32) -> impl Iterator<Item = u32> {
    struct BitIter {
        value: u32,
    }

    impl Iterator for BitIter {
        type Item = u32;
        fn next(&mut self) -> Option<u32> {
            match self.value.trailing_zeros() {
                32 => None,
                index => {
                    self.value &= !(1 << index);
                    Some(index)
                }
            }
        }
    }

    BitIter { value }
}
