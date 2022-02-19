use std::path::Path;

use treasury_import::{Dependencies, ImportError, Importer, Sources};

pub struct CopyImporter;

impl Importer for CopyImporter {
    fn name(&self) -> &str;
    fn formats(&self) -> &[&str];
    fn extensions(&self) -> &[&str];
    fn target(&self) -> &str;

    fn import(
        &self,
        source: &Path,
        output: &Path,
        _sources: &mut (impl Sources + ?Sized),
        _dependencies: &mut (impl Dependencies + ?Sized),
    ) -> Result<(), ImportError> {
        match std::fs::copy(source_path, output_path) {
            Ok(_) => Ok(()),
            Err(err) => Err(ImportError::Other {
                reason: format!(
                    "Failed to copy '{}' to '{}'. {:#}",
                    source.display(),
                    output.display(),
                    err
                ),
            }),
        }
    }
}
