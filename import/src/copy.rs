use std::path::Path;

use treasury_import::{Dependencies, ImportError, Importer, Sources};

pub struct CopyImporter;

impl Importer for CopyImporter {
    fn import(
        &self,
        source_path: &Path,
        output_path: &Path,
        _sources: &impl Sources,
        _dependencies: &impl Dependencies,
    ) -> Result<(), ImportError> {
        match std::fs::copy(source_path, output_path) {
            Ok(_) => Ok(()),
            Err(err) => Err(ImportError::Other {
                reason: err.to_string(),
            }),
        }
    }
}
