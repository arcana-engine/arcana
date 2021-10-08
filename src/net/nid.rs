use std::{mem::align_of, num::NonZeroU64};

use alkahest::{Pack, Schema, SchemaUnpack};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct NetId(pub NonZeroU64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Zero NetId unpacked")]
pub struct ZeroNetIdError;

impl SchemaUnpack<'_> for NetId {
    type Unpacked = Result<Self, ZeroNetIdError>;
}

impl Schema for NetId {
    type Packed = u64;

    fn align() -> usize {
        align_of::<u64>()
    }

    fn unpack<'a>(packed: u64, _input: &'a [u8]) -> Result<Self, ZeroNetIdError> {
        NonZeroU64::new(packed).map(NetId).ok_or(ZeroNetIdError)
    }
}

impl Pack<NetId> for NetId {
    fn pack(self, _offset: usize, _output: &mut [u8]) -> (u64, usize) {
        (self.0.get(), 0)
    }
}

impl Pack<NetId> for &'_ NetId {
    fn pack(self, _offset: usize, _output: &mut [u8]) -> (u64, usize) {
        (self.0.get(), 0)
    }
}
