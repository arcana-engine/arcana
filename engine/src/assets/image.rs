use std::sync::Arc;

use goods::TrivialAsset;
use rapid_qoi::DecodeError;

#[derive(Clone)]
pub struct QoiImage {
    pub qoi: rapid_qoi::Qoi,
    pub pixels: Arc<[u8]>,
}

impl TrivialAsset for QoiImage {
    type Error = DecodeError;

    fn name() -> &'static str {
        "qoi"
    }

    fn decode(bytes: Box<[u8]>) -> Result<Self, DecodeError> {
        rapid_qoi::Qoi::decode_alloc(&bytes).map(|(qoi, pixels)| QoiImage {
            qoi,
            pixels: pixels.into(),
        })
    }
}
