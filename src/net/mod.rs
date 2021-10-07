use std::{collections::HashMap, marker::PhantomData, mem::align_of, num::NonZeroU64};

use alkahest::{Pack, Schema, SchemaUnpack};

pub use astral::client_server::PlayerId;
use hecs::Entity;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "client")]
pub mod client;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct NetId(pub NonZeroU64);

impl SchemaUnpack<'_> for NetId {
    type Unpacked = Option<NetId>;
}

impl Schema for NetId {
    type Packed = u64;

    fn align() -> usize {
        align_of::<u64>()
    }

    fn unpack<'a>(packed: u64, _input: &'a [u8]) -> Option<NetId> {
        NonZeroU64::new(packed).map(NetId)
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

pub struct ReplicaSerde<T>(PhantomData<fn() -> T>);

pub struct ReplicaPrefabSerde<T>(PhantomData<fn() -> T>);

pub struct EntityMapper {
    entity_by_id: HashMap<NetId, Entity>,
}

impl EntityMapper {
    fn new() -> Self {
        EntityMapper {
            entity_by_id: HashMap::new(),
        }
    }
}
