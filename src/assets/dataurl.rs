use {
    super::source::{AssetData, Source},
    std::future::{ready, Ready},
    url::Url,
};

/// Loads asset data directly from URL with [data scheme](https://tools.ietf.org/html/rfc2397).
#[derive(Debug)]
pub struct DataUrlSource;

#[derive(Debug, thiserror::Error)]
pub enum DataUrlError {
    #[error("Url head us not followed by the data")]
    MissingData,

    #[error("Failed to decode from base64")]
    DecodeBase64Error {
        #[from]
        source: base64::DecodeError,
    },
}

impl Source for DataUrlSource {
    type Error = DataUrlError;
    type Fut = Ready<Result<Option<AssetData>, DataUrlError>>;

    fn load(&self, key: &str) -> Ready<Result<Option<AssetData>, DataUrlError>> {
        let url = match key.parse::<Url>() {
            Ok(url) => url,
            Err(_) => return ready(Ok(None)),
        };

        if url.scheme() != "data" {
            return ready(Ok(None));
        }

        let dataurl = url.path().as_bytes();
        match dataurl.iter().position(|&b| b == b',') {
            None => ready(Err(DataUrlError::MissingData)),
            Some(comma) => {
                let data = &dataurl[comma + 1..];

                match dataurl[..comma] {
                    [.., b';', b'b', b'a', b's', b'e', b'6', b'4'] => match base64::decode(data) {
                        Ok(bytes) => ready(Ok(Some(AssetData {
                            bytes: bytes.into(),
                            version: 0,
                        }))),
                        Err(err) => ready(Err(DataUrlError::DecodeBase64Error { source: err })),
                    },
                    _ => {
                        let data = percent_encoding::percent_decode(data)
                            .decode_utf8()
                            .unwrap();
                        ready(Ok(Some(AssetData {
                            bytes: data.into_owned().into_boxed_str().into(),
                            version: 0,
                        })))
                    }
                }
            }
        }
    }

    fn update(&self, _key: &str, _version: u64) -> Ready<Result<Option<AssetData>, DataUrlError>> {
        ready(Ok(None))
    }
}
