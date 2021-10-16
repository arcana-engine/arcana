use std::{mem::align_of, num::NonZeroU64};

use alkahest::{FixedUsize, Schema, SchemaUnpack};
pub use astral::client_server::PlayerId;

pub use self::{mapper::EntityMapper, nid::NetId};

mod mapper;
mod nid;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "client")]
pub mod client;

struct IdGen {
    next: NonZeroU64,
}

impl IdGen {
    pub fn new() -> Self {
        IdGen {
            next: NonZeroU64::new(1).unwrap(),
        }
    }

    pub fn gen_nid(&mut self) -> NetId {
        NetId(self.gen())
    }

    pub fn gen_pid(&mut self) -> PlayerId {
        PlayerId(self.gen())
    }

    pub fn gen(&mut self) -> NonZeroU64 {
        let id = self.next;
        let next = self
            .next
            .get()
            .checked_add(1)
            .expect("u64 increment overflow");

        self.next = NonZeroU64::new(next).unwrap();

        id
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EntityHeader<B> {
    nid: NetId,
    mask: B,
}

struct WorldSchema;

#[derive(Clone, Copy)]
#[repr(C)]
struct WorldPacked {
    offset: FixedUsize,
    updated: FixedUsize,
    removed: FixedUsize,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct WorldUnpacked<'a> {
    raw: &'a [u8],
    updated: usize,
    removed: usize,
}

unsafe impl bytemuck::Zeroable for WorldPacked {}
unsafe impl bytemuck::Pod for WorldPacked {}

impl<'a> SchemaUnpack<'a> for WorldSchema {
    type Unpacked = WorldUnpacked<'a>;
}

impl Schema for WorldSchema {
    type Packed = WorldPacked;

    fn align() -> usize {
        align_of::<FixedUsize>()
    }

    fn unpack<'a>(packed: WorldPacked, input: &'a [u8]) -> WorldUnpacked<'a> {
        let offset = packed.offset as usize;
        let raw = &input[offset..];
        let updated = packed.updated as usize;
        let removed = packed.removed as usize;

        WorldUnpacked {
            raw,
            updated,
            removed,
        }
    }
}

struct InputSchema;

#[derive(Clone, Copy)]
#[repr(C)]
struct InputPacked {
    offset: FixedUsize,
    len: FixedUsize,
}

unsafe impl bytemuck::Zeroable for InputPacked {}
unsafe impl bytemuck::Pod for InputPacked {}

impl<'a> SchemaUnpack<'a> for InputSchema {
    type Unpacked = &'a [u8];
}

impl Schema for InputSchema {
    type Packed = InputPacked;

    fn align() -> usize {
        align_of::<FixedUsize>()
    }

    fn unpack<'a>(packed: InputPacked, input: &'a [u8]) -> &'a [u8] {
        let offset = packed.offset as usize;
        let len = packed.len as usize;
        &input[offset..][..len]
    }
}
