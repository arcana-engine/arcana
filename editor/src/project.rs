use std::path::PathBuf;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectConfig {
    /// Game config.
    pub game: arcana::cfg::Config,
}

/// Structure that describes arcana project.
pub struct ArcanaProject {
    /// Path to root directory of the project.
    /// Most other paths are relative to the root.
    pub root: PathBuf,

    /// Path to project file.
    /// Usually named `Arcana.toml`
    /// Most often is `<root>/Arcana.toml`
    pub path: PathBuf,
}

impl ArcanaProject {}
