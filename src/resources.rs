use crate::noophash::NoopHasherBuilder;

use {
    hashbrown::hash_map::{Entry, HashMap},
    std::any::{Any, TypeId},
};

/// Resources map.
/// Can contain up to one instance of a type.
pub struct Res {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>, NoopHasherBuilder>,
}

impl Res {
    /// Returns new empty resources map.
    pub fn new() -> Self {
        Res {
            map: HashMap::with_hasher(NoopHasherBuilder),
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

    /// Removes resource and returns it.
    pub fn remove<T: 'static>(&mut self) -> Option<Box<T>> {
        self.map
            .remove(&TypeId::of::<T>())
            .map(|b| b.downcast().unwrap())
    }

    /// Query multiple resources at once.
    /// Items queried are expected in the `Resources`.
    /// To query optionally, wrap reference in `Option`.
    pub fn query<'a, Q>(&'a mut self) -> Q::Item
    where
        Q: Query<'a>,
    {
        Q::get(self)
    }
}

pub trait Query<'a> {
    type Item;
    fn get(res: &'a mut Res) -> Self::Item;
}

pub unsafe trait Fetch<'a>: Sized {
    fn ty() -> TypeId;
    fn mutable() -> bool;
    unsafe fn get(res: *mut Res) -> Self;
}

unsafe impl<'a, T: 'static> Fetch<'a> for &'a T {
    fn ty() -> TypeId {
        TypeId::of::<T>()
    }
    fn mutable() -> bool {
        false
    }
    unsafe fn get(res: *mut Res) -> &'a T {
        let r = (&*res)
            .map
            .get(&TypeId::of::<T>())
            .expect("Resource expected");
        r.downcast_ref().unwrap()
    }
}

unsafe impl<'a, T: 'static> Fetch<'a> for &'a mut T {
    fn ty() -> TypeId {
        TypeId::of::<T>()
    }
    fn mutable() -> bool {
        true
    }
    unsafe fn get(res: *mut Res) -> &'a mut T {
        let r = (&mut *res)
            .map
            .get_mut(&TypeId::of::<T>())
            .expect("Resource expected");
        r.downcast_mut().unwrap()
    }
}

unsafe impl<'a, T: 'static> Fetch<'a> for Option<&'a T> {
    fn ty() -> TypeId {
        TypeId::of::<T>()
    }
    fn mutable() -> bool {
        false
    }
    unsafe fn get(res: *mut Res) -> Option<&'a T> {
        let r = (&*res).map.get(&TypeId::of::<T>())?;
        Some(r.downcast_ref().unwrap())
    }
}

unsafe impl<'a, T: 'static> Fetch<'a> for Option<&'a mut T> {
    fn ty() -> TypeId {
        TypeId::of::<T>()
    }
    fn mutable() -> bool {
        true
    }
    unsafe fn get(res: *mut Res) -> Option<&'a mut T> {
        let r = (&mut *res).map.get_mut(&TypeId::of::<T>())?;
        Some(r.downcast_mut().unwrap())
    }
}

unsafe trait QueryValid<'a> {
    fn is_valid() -> bool;
}

macro_rules! for_tuple {
    () => {
        for_tuple!(for A B C D E F G H I J K L M N O P);
    };

    (for) => {
        for_tuple!(impl);
    };

    (for $head:ident $($tail:ident)*) => {
        for_tuple!(for $($tail)*);
        for_tuple!(impl $head $($tail)*);
    };

    (impl) => {
        impl<'a> Query<'a> for () {
            type Item = ();
            fn get(_res: &'a mut Res) {}
        }
    };

    (impl $($a:ident)+) => {
        unsafe impl<'a, $($a),+> QueryValid<'a> for ($($a,)+) where $($a: Fetch<'a>,)+ {
            #[inline]
            fn is_valid() -> bool {
                let mut pairs: &[_] = &[$(($a::ty(), $a::mutable(),),)+];
                while let [(ty, mutable), rest @ ..] = pairs {
                    let mut rest = rest;
                    if let [(head_ty, head_mutable), tail @ ..] = rest {
                        if (*mutable || *head_mutable) && (ty == head_ty) {
                            return false;
                        }
                        rest = tail;
                    }
                    pairs = rest;
                }
                true
            }
        }

        impl<'a, $($a),+> Query<'a> for ($($a,)+) where $($a: Fetch<'a>,)+ {
            type Item = ($($a,)+);

            fn get(res: &'a mut Res) -> ($($a,)+) {
                assert!(<Self as QueryValid>::is_valid());
                unsafe { ($( $a::get(res), )+) }
            }
        }
    };
}

for_tuple!();
