use std::{
    error::Error,
    fmt, io,
    path::{Path, PathBuf},
};

use futures::future::BoxFuture;
use goods::{
    source::{AssetData, Source},
    AssetId,
};
use treasury_store::{Treasury, TreasuryInfo};

pub struct TreasurySource {
    store: Treasury,
}

impl TreasurySource {
    pub fn new(base: &Path, info: TreasuryInfo) -> eyre::Result<Self> {
        Ok(TreasurySource {
            store: Treasury::new(base, info)?,
        })
    }
}

#[derive(Debug)]
pub enum TreasuryError {
    Report(eyre::Report),
    File { path: PathBuf, error: io::Error },
}

impl fmt::Display for TreasuryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreasuryError::Report(report) => fmt::Display::fmt(report, f),
            TreasuryError::File { path, error } => {
                write!(f, "'{}' error. {:#}", path.display(), error)
            }
        }
    }
}

impl Error for TreasuryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TreasuryError::Report(report) => Some(report.as_ref()),
            TreasuryError::File { error, .. } => Some(error),
        }
    }
}

impl Source for TreasurySource {
    type Error = TreasuryError;

    fn find(&self, path: &str, asset: &str) -> BoxFuture<Option<AssetId>> {
        let path = path.to_owned();
        let asset = asset.to_owned();
        Box::pin(async move {
            match self.store.find_asset(&path, &asset).await {
                Ok(Some((id, _path))) => Some(AssetId(id.value())),
                Ok(None) => None,
                Err(err) => {
                    tracing::error!("Failed to fetch asset {}@{} from treasury", asset, path);
                    None
                }
            }
        })
    }

    fn load(&self, id: AssetId) -> BoxFuture<Result<Option<AssetData>, TreasuryError>> {
        Box::pin(async move {
            match self.store.fetch(id.0.into()).await {
                None => Ok(None),
                Some(path) => match std::fs::read(&path) {
                    Err(error) => Err(TreasuryError::File { path, error }),
                    Ok(data) => Ok(Some(AssetData {
                        bytes: data.into_boxed_slice(),
                        version: 0,
                    })),
                },
            }
        })
    }

    fn update(
        &self,
        id: AssetId,
        version: u64,
    ) -> BoxFuture<Result<Option<AssetData>, TreasuryError>> {
        Box::pin(async { Ok(None) })
    }
}
