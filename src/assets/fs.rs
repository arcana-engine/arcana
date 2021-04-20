use {
    super::source::{AssetData, Source},
    std::{
        future::{ready, Ready},
        io::{Error, Read as _},
        path::{Path, PathBuf},
    },
};

/// Loads assets from file-system.
#[derive(Debug)]
pub struct FsSource {
    root: Option<PathBuf>,
}

impl FsSource {
    /// Returns new [`FsSource`] instance with not root provided.
    /// Root-less [`FsSource`] will interpret asset key as a path.
    pub const fn new() -> Self {
        FsSource { root: None }
    }

    /// Returns new [`FsSource`] instance with root provided.
    /// Rooted [`FsSource`] will interpret asset key as a path relative to root.
    pub const fn with_root(root: PathBuf) -> Self {
        FsSource { root: Some(root) }
    }
}

impl Source for FsSource {
    type Error = Error;
    type Fut = Ready<Result<Option<AssetData>, Error>>;

    #[tracing::instrument]
    fn load(&self, key: &str) -> Ready<Result<Option<AssetData>, Error>> {
        match &self.root {
            Some(root) => ready(load(key.as_ref())),
            None => ready(load(key.as_ref())),
        }
    }

    #[tracing::instrument]
    fn update(&self, key: &str, version: u64) -> Ready<Result<Option<AssetData>, Error>> {
        match &self.root {
            Some(root) => ready(update(key.as_ref(), version)),
            None => ready(update(key.as_ref(), version)),
        }
    }
}

fn load(path: &Path) -> Result<Option<AssetData>, Error> {
    let result = std::fs::File::open(path);

    match result {
        Ok(mut file) => {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            let bytes = bytes.into_boxed_slice();

            if let Ok(metadata) = file.metadata() {
                if let Ok(modified) = metadata.modified() {
                    let duration = modified
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap();
                    return Ok(Some(AssetData {
                        bytes,
                        version: duration.as_secs(),
                    }));
                }
            }

            Ok(Some(AssetData {
                bytes,
                version: std::time::SystemTime::UNIX_EPOCH
                    .elapsed()
                    .unwrap()
                    .as_secs(),
            }))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::Other => Ok(None),
        Err(err) => Err(err),
    }
}

fn update(path: &Path, version: u64) -> Result<Option<AssetData>, Error> {
    let file = std::fs::File::open(path);
    let mut file = file?;

    if let Ok(metadata) = file.metadata() {
        if let Ok(modified) = metadata.modified() {
            let duration = modified
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap();

            let new_version = duration.as_secs();

            if new_version > version {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                let bytes = bytes.into_boxed_slice();
                return Ok(Some(AssetData {
                    bytes,
                    version: new_version,
                }));
            }
        }
    }

    Ok(None)
}
