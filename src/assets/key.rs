use {
    super::asset::Asset,
    std::{
        any::TypeId,
        hash::{Hash, Hasher},
    },
};

#[derive(Clone, PartialEq, Eq)]
pub struct Key {
    type_id: TypeId,
    key: Box<str>,
}

impl Key {
    pub fn new<A: Asset>(key: Box<str>) -> Self {
        Key {
            type_id: TypeId::of::<A>(),
            key,
        }
    }

    pub fn eq_key<A: Asset>(&self, key: &str) -> bool {
        self.type_id == TypeId::of::<A>() && *self.key == *key
    }
}

pub fn hash_key<A, H>(key: &str, state: &mut H)
where
    A: Asset,
    H: Hasher,
{
    TypeId::of::<A>().hash(state);
    key.hash(state);
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.key.hash(state);
    }
}
