use std::{marker::PhantomData, mem::align_of};

use alkahest::{Pack, Schema, SchemaUnpack};

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "client")]
pub mod client;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct NetId(pub u64);

impl SchemaUnpack<'_> for NetId {
    type Unpacked = Self;
}

impl Schema for NetId {
    type Packed = u64;
    fn align() -> usize {
        align_of::<u64>()
    }
    fn unpack<'a>(packed: u64, _input: &'a [u8]) -> Self {
        NetId(packed)
    }
}

impl Pack<NetId> for NetId {
    fn pack(self, _offset: usize, _output: &mut [u8]) -> (u64, usize) {
        (self.0, 0)
    }
}

impl Pack<NetId> for &'_ NetId {
    fn pack(self, _offset: usize, _output: &mut [u8]) -> (u64, usize) {
        (self.0, 0)
    }
}
pub struct ReplicaSerde<T>(PhantomData<fn() -> T>);
