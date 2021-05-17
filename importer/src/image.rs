use {
    std::{error::Error, io::Read, path::Path},
    treasury_import::{Importer, Registry},
};

pub struct ImageImporter;

impl Importer for ImageImporter {
    fn name(&self) -> &str {
        "arcana.image"
    }

    fn import(
        &self,
        source_path: &Path,
        native_path: &Path,
        _registry: &mut dyn Registry,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut source = std::fs::File::open(source_path)?;

        // # Find out image format.
        //
        // 1.Read few first bytes.
        let mut head = [0; 32];
        let mut read_total = 0;

        // This loop is expected to be executed once.
        while read_total < 32 {
            let read = source.read(&mut head[read_total..])?;
            if read == 0 {
                // 1.1 Huh. That's some small image. Try to load it then.
                image::load_from_memory(&head[..read_total])?
                    .save_with_format(native_path, image::ImageFormat::Png)?;
                return Ok(());
            }
            read_total += read;
        }

        // 2. Guess format
        let format = image::guess_format(&head)?;
        match format {
            image::ImageFormat::Png => {
                std::fs::hard_link(source_path, native_path)?;
                Ok(())
            }
            format => {
                let mut bytes = vec![];
                bytes.extend_from_slice(&head);
                source.read_to_end(&mut bytes)?;

                let dyn_image = image::load_from_memory_with_format(&bytes, format)?;
                dyn_image.save_with_format(native_path, image::ImageFormat::Png)?;
                Ok(())
            }
        }
    }
}
