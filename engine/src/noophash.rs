use std::hash::{BuildHasher, Hasher};

/// Builder for `NopHasher` hashers.
pub struct NoopHasherBuilder;

/// Hasher that perform no operations.
/// Can be used for keys that are already hashed,
/// such as [`TypeId`].
pub struct NopHasher(u64);

impl BuildHasher for NoopHasherBuilder {
    type Hasher = NopHasher;

    fn build_hasher(&self) -> NopHasher {
        NopHasher(0)
    }
}

impl Hasher for NopHasher {
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        let mut copy = [0u8; 8];
        copy[..bytes.len().min(8)].copy_from_slice(bytes);
        self.0 = u64::from_ne_bytes(copy);
    }

    #[inline(always)]
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }
}
