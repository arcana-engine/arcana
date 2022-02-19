use std::path::{Path, PathBuf};

use arcana_time::TimeSpan;

const CONFIG_DEFAULT_NAME: &'static str = "Arcana.toml";

#[derive(serde::Deserialize)]
#[cfg(feature = "asset-pipeline")]
pub struct TreasuryConfig {
    pub base: Box<Path>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub artifacts: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub external: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub temp: Option<PathBuf>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub importers: Vec<PathBuf>,
}

#[allow(unused)]
#[derive(serde::Deserialize)]
pub struct Config {
    #[cfg(feature = "asset-pipeline")]
    #[serde(default)]
    pub treasury: Option<TreasuryConfig>,

    #[serde(default = "default_teardown_timeout")]
    pub teardown_timeout: TimeSpan,

    #[serde(default = "default_main_step")]
    pub main_step: TimeSpan,

    #[serde(default = "default_root")]
    pub root: Box<Path>,
}

impl Config {
    pub fn new(root: PathBuf) -> Self {
        Config {
            #[cfg(feature = "asset-pipeline")]
            treasury: None,
            teardown_timeout: default_teardown_timeout(),
            main_step: default_main_step(),
            root: root.into(),
        }
    }

    pub fn load(path: &Path) -> eyre::Result<Self> {
        load_config(path)
    }

    pub fn load_default() -> Self {
        load_default_config()
    }
}

fn default_teardown_timeout() -> TimeSpan {
    TimeSpan::from_seconds(5)
}

fn default_main_step() -> TimeSpan {
    TimeSpan::from_millis(20)
}

fn default_root() -> Box<Path> {
    PathBuf::new().into_boxed_path()
}

#[tracing::instrument]
fn load_config(path: &Path) -> eyre::Result<Config> {
    let bytes = std::fs::read(&path)?;
    let mut cfg: Config = toml::from_slice(&bytes)?;

    if *cfg.root == *Path::new("") {
        if let Some(cfg_dir) = path.parent() {
            cfg.root = cfg_dir.to_owned().into_boxed_path();
        }
    } else if let Ok(path) = dunce::canonicalize(&cfg.root) {
        cfg.root = path.into_boxed_path();
    }

    Ok(cfg)
}

fn try_load_default_config() -> eyre::Result<Config> {
    tracing::debug!("Loading config");

    match lookup_relpath(Path::new(CONFIG_DEFAULT_NAME)) {
        Some(path) => load_config(&path),
        None => Err(eyre::eyre!("Failed to locate config file")),
    }
}

fn load_default_config() -> Config {
    match try_load_default_config() {
        Ok(cfg) => cfg,
        Err(err) => {
            tracing::debug!("Config file not found. {:#}", err);
            Config::new(".".into())
        }
    }
}

#[allow(unused)]
fn lookup_in_current_dir(relpath: &Path) -> Option<PathBuf> {
    let cd = std::env::current_dir().ok()?;

    for dir in cd.ancestors() {
        let candidate = dir.join(relpath);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[allow(unused)]
fn lookup_in_binary_dir(relpath: &Path) -> Option<PathBuf> {
    let ce = std::env::current_exe().ok()?;

    let mut ancestors = ce.ancestors();
    ancestors.next();

    for dir in ancestors {
        let candidate = dir.join(relpath);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[allow(unused)]
fn lookup_relpath(relpath: &Path) -> Option<PathBuf> {
    if let Some(path) = lookup_in_current_dir(relpath) {
        return Some(path);
    }
    if let Some(path) = lookup_in_binary_dir(relpath) {
        return Some(path);
    }
    None
}
