use std::path::PathBuf;

/// Structure that describes arcana project.
pub struct ArcanaProject {
    /// Path to metadata file. Usually named `Arcana.toml`
    pub path: PathBuf,
}

impl ArcanaProject {
    /// Creates new arcana project.
    /// `path` must be non-existing.
    pub fn create(path: &Path) -> Self {}
}
