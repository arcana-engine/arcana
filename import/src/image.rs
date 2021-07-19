use {
    eyre::WrapErr,
    goods_treasury_import::{Importer, Registry},
    std::{io::Read, path::Path},
};

pub struct ImageImporter;

impl Importer for ImageImporter {
    fn name(&self) -> &str {
        "arcana.image"
    }

    fn source(&self) -> &str {
        "image"
    }

    fn native(&self) -> &str {
        "rgba.png"
    }

    fn import(
        &self,
        source_path: &Path,
        native_path: &Path,
        _registry: &mut dyn Registry,
    ) -> eyre::Result<()> {
        let mut source = std::fs::File::open(source_path)
            .wrap_err_with(|| format!("Failed to open file '{}'", source_path.display()))?;

        // # Find out image format.
        //
        // 1.Read few first bytes.
        let mut head = [0; 32];
        let mut read_total = 0;

        // This loop is expected to be executed once.
        while read_total < 32 {
            let read = source
                .read(&mut head[read_total..])
                .wrap_err_with(|| format!("Failed to read file '{}'", source_path.display()))?;
            if read == 0 {
                // 1.1 Huh. That's some small image. Try to load it then.
                image::load_from_memory(&head[..read_total])
                    .wrap_err_with(|| {
                        format!("Failed to load image from file '{}'", source_path.display())
                    })?
                    .save_with_format(native_path, image::ImageFormat::Png)
                    .wrap_err_with(|| {
                        format!("Failed to save image to file '{}'", native_path.display())
                    })?;
                return Ok(());
            }
            read_total += read;
        }

        // 2. Guess format
        let format = image::guess_format(&head).wrap_err_with(|| {
            format!(
                "Failed to guess image format from file '{}'",
                source_path.display()
            )
        })?;
        match format {
            image::ImageFormat::Png => {
                std::fs::copy(source_path, native_path).wrap_err_with(|| {
                    format!(
                        "Failed to copy image file '{}' to '{}'",
                        source_path.display(),
                        native_path.display()
                    )
                })?;
                Ok(())
            }
            format => {
                let mut bytes = vec![];
                bytes.extend_from_slice(&head);
                source.read_to_end(&mut bytes).wrap_err_with(|| {
                    format!("Failed to read image file '{}'", source_path.display())
                })?;

                let dyn_image =
                    image::load_from_memory_with_format(&bytes, format).wrap_err_with(|| {
                        format!("Failed to read image file '{}'", source_path.display())
                    })?;
                dyn_image
                    .save_with_format(native_path, image::ImageFormat::Png)
                    .wrap_err_with(|| {
                        format!("Failed to save image file '{}'", native_path.display())
                    })?;
                Ok(())
            }
        }
    }
}
