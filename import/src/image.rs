use treasury_import::{Dependencies, ImportError, Sources};

use {
    std::{io::Read, path::Path},
    treasury_import::Importer,
};

pub struct ImageImporter;

impl Importer for ImageImporter {
    fn import(
        &self,
        source_path: &Path,
        output_path: &Path,
        _sources: &impl Sources,
        _dependencies: &impl Dependencies,
    ) -> Result<(), ImportError> {
        let mut source = std::fs::File::open(source_path).map_err(|err| ImportError::Other {
            reason: format!("Failed to open file '{}'. {:#}", source_path.display(), err),
        })?;

        const QOI_HEADER_SIZE: usize = 14;

        // # Find out image format.
        //
        // 1.Read few first bytes.
        let mut head = [0; QOI_HEADER_SIZE];
        let mut read_total = 0;

        // This loop is expected to be executed once.
        let image = loop {
            if read_total < head.len() {
                let read =
                    source
                        .read(&mut head[read_total..])
                        .map_err(|err| ImportError::Other {
                            reason: format!(
                                "Failed to read file '{}'. {:#}",
                                source_path.display(),
                                err
                            ),
                        })?;

                if read == 0 {
                    // 1.1 Huh. That's some small image. Try to load it then.
                    break image::load_from_memory(&head[..read_total]).map_err(|err| {
                        ImportError::Other {
                            reason: format!(
                                "Failed to load image from file '{}'. {:#}",
                                source_path.display(),
                                err
                            ),
                        }
                    })?;
                }
                read_total += read;
            } else {
                if rapid_qoi::Qoi::decode_header(&head).is_ok() {
                    drop(source);

                    std::fs::copy(source_path, output_path).map_err(|err| ImportError::Other {
                        reason: format!(
                            "Failed to copy image file '{}' to '{}'. {:#}",
                            source_path.display(),
                            output_path.display(),
                            err,
                        ),
                    })?;

                    return Ok(());
                } else {
                    let format = image::guess_format(&head).map_err(|err| ImportError::Other {
                        reason: format!(
                            "Failed to guess image format from file '{}'. {:#}",
                            source_path.display(),
                            err
                        ),
                    })?;

                    let mut bytes = vec![];
                    bytes.extend_from_slice(&head);
                    source
                        .read_to_end(&mut bytes)
                        .map_err(|err| ImportError::Other {
                            reason: format!(
                                "Failed to read image file '{}'. {:#}",
                                source_path.display(),
                                err,
                            ),
                        })?;

                    break image::load_from_memory_with_format(&bytes, format).map_err(|err| {
                        ImportError::Other {
                            reason: format!(
                                "Failed to read image file '{}'. {:#}",
                                source_path.display(),
                                err
                            ),
                        }
                    })?;
                }
            }
        };

        match image.color() {
            image::ColorType::Rgba8 | image::ColorType::Rgba16 => {
                let image = image.into_rgba8();

                let qoi = rapid_qoi::Qoi {
                    width: image.width(),
                    height: image.height(),
                    colors: rapid_qoi::Colors::SrgbLinA,
                }
                .encode_alloc(image.as_raw())
                .map_err(|err| ImportError::Other {
                    reason: format!("Failed to encode QOI image. {:#}", err),
                })?;

                std::fs::write(output_path, &qoi).map_err(|err| ImportError::Other {
                    reason: format!(
                        "Failed to save image format from file '{}'. {:#}",
                        source_path.display(),
                        err
                    ),
                })?;
            }
            _ => {
                let image = image.into_rgb8();

                let qoi = rapid_qoi::Qoi {
                    width: image.width(),
                    height: image.height(),
                    colors: rapid_qoi::Colors::Srgb,
                }
                .encode_alloc(image.as_raw())
                .map_err(|err| ImportError::Other {
                    reason: format!("Failed to encode QOI image. {:#}", err),
                })?;

                std::fs::write(output_path, &qoi).map_err(|err| ImportError::Other {
                    reason: format!(
                        "Failed to save image format from file '{}'. {:#}",
                        source_path.display(),
                        err
                    ),
                })?;
            }
        }
        Ok(())
    }
}
