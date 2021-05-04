use {
    hashbrown::hash_map::{Entry, HashMap},
    std::{
        any::{Any, TypeId},
        hash::{BuildHasher, Hasher},
    },
};

/// Builder for `NopHasher` hashers.
pub struct NopHasherBuilder;

/// Hasher that perform no operations.
/// Can be used for keys that are already hashed,
/// such as [`TypeId`].
pub struct NopHasher(u64);

impl BuildHasher for NopHasherBuilder {
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
}

/// Resources map.
/// Can contain up to one instance of a type.
pub struct Res {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>, NopHasherBuilder>,
}

impl Res {
    /// Returns new empty resources map.
    pub fn new() -> Self {
        Res {
            map: HashMap::with_hasher(NopHasherBuilder),
        }
    }

    /// Inserts value into the map.
    /// Returns old value of the same type if one was added into map before.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> Option<T> {
        match self.map.entry(TypeId::of::<T>()) {
            Entry::Occupied(mut entry) => {
                let old = entry.get_mut().downcast_mut().unwrap();
                Some(std::mem::replace(old, value))
            }
            Entry::Vacant(entry) => {
                entry.insert(Box::new(value));
                None
            }
        }
    }

    /// Returns reference to value in the map.
    /// Returns `None` if value of requested type was not added into map before.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .map(|b| b.downcast_ref().unwrap())
    }

    /// Returns mutable reference to value in the map.
    /// Returns `None` if value of requested type was not added into map before.
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .map(|b| b.downcast_mut().unwrap())
    }

    /// Returns mutable reference to value in the map.
    /// Executes provided closure and adds one into map if vale of requested
    /// type was not added into map before.
    pub fn with<T: Send + Sync + 'static>(&mut self, f: impl FnOnce() -> T) -> &mut T {
        self.map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(f()))
            .downcast_mut()
            .unwrap()
    }

    /// Returns mutable reference to value in the map.
    /// Executes provided closure and adds one into map if vale of requested
    /// type was not added into map before.
    ///
    /// Unlike [`Resources::with`] closure may fail returning error
    /// which will be propagated back to caller.
    pub fn try_with<T: Send + Sync + 'static>(
        &mut self,
        f: impl FnOnce() -> eyre::Result<T>,
    ) -> eyre::Result<&mut T> {
        match self.map.entry(TypeId::of::<T>()) {
            Entry::Occupied(entry) => Ok(entry.into_mut().downcast_mut().unwrap()),
            Entry::Vacant(entry) => {
                let value = f()?;
                Ok(entry.insert(Box::new(value)).downcast_mut().unwrap())
            }
        }
    }
}
