use {
    eyre::WrapErr,
    goods_treasury_import::{Importer, Registry},
    std::path::Path,
};

pub struct TileSetImporter;

impl Importer for TileSetImporter {
    fn name(&self) -> &str {
        "tile-set"
    }

    fn source(&self) -> &str {
        "arcana.tile-set"
    }

    fn native(&self) -> &str {
        "arcana.tile-set"
    }

    fn import(
        &self,
        source_path: &Path,
        native_path: &Path,
        _registry: &mut dyn Registry,
    ) -> eyre::Result<()> {
        std::fs::copy(source_path, native_path).wrap_err_with(|| {
            format!(
                "Failed to copy tile set file '{}' to '{}'",
                source_path.display(),
                native_path.display()
            )
        })?;

        Ok(())
    }
}

pub struct TileMapImporter;

impl Importer for TileMapImporter {
    fn name(&self) -> &str {
        "tile-map"
    }

    fn source(&self) -> &str {
        "arcana.tile-map"
    }

    fn native(&self) -> &str {
        "arcana.tile-map"
    }

    fn import(
        &self,
        source_path: &Path,
        native_path: &Path,
        _registry: &mut dyn Registry,
    ) -> eyre::Result<()> {
        std::fs::copy(source_path, native_path).wrap_err_with(|| {
            format!(
                "Failed to copy tile set file '{}' to '{}'",
                source_path.display(),
                native_path.display()
            )
        })?;

        Ok(())
    }
}
