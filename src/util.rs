use {
    byteorder::ByteOrder,
    sierra::{mat, vec, vec2, vec3, vec4},
    std::mem::size_of,
};

/// Read value from raw bytes.
pub trait FromBytes: Default {
    /// Loads value from raw bytes slice.
    /// This function may expect that bytes len equals size of the type.
    ///
    /// # Panics
    ///
    /// This function is expected to panic if bytes len is invalid.
    fn from_bytes<E: ByteOrder>(bytes: &[u8]) -> Self;
}

impl FromBytes for u8 {
    fn from_bytes<E: ByteOrder>(bytes: &[u8]) -> Self {
        bytes[0]
    }
}

impl FromBytes for u16 {
    fn from_bytes<E: ByteOrder>(bytes: &[u8]) -> Self {
        E::read_u16(bytes)
    }
}

impl FromBytes for u32 {
    fn from_bytes<E: ByteOrder>(bytes: &[u8]) -> Self {
        E::read_u32(bytes)
    }
}

impl FromBytes for u64 {
    fn from_bytes<E: ByteOrder>(bytes: &[u8]) -> Self {
        E::read_u64(bytes)
    }
}

impl FromBytes for f32 {
    fn from_bytes<E: ByteOrder>(bytes: &[u8]) -> Self {
        E::read_f32(bytes)
    }
}

impl FromBytes for f64 {
    fn from_bytes<E: ByteOrder>(bytes: &[u8]) -> Self {
        E::read_f64(bytes)
    }
}
