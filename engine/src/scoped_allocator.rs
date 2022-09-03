use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use edict::{
    archetype::Archetype,
    query::Access,
    system::{ActionQueue, FnArg, FnArgCache, FnArgGet},
    world::World,
};
use scoped_arena::Scope;

pub struct ScopedAllocator {
    scope: Scope<'static>,
}

pub struct ScopedAllocatorCache {
    allocator: ScopedAllocator,
}

/// Resets the given allocator scope before it could be sent to another thread.
unsafe impl Send for ScopedAllocatorCache {}

impl Deref for ScopedAllocator {
    type Target = Scope<'static>;

    #[inline]
    fn deref(&self) -> &Scope<'static> {
        &self.scope
    }
}

impl DerefMut for ScopedAllocator {
    #[inline]
    fn deref_mut(&mut self) -> &mut Scope<'static> {
        &mut self.scope
    }
}

impl FnArg for &mut ScopedAllocator {
    type Cache = ScopedAllocatorCache;
}

unsafe impl<'a> FnArgGet<'a> for ScopedAllocatorCache {
    type Arg = &'a mut ScopedAllocator;

    #[inline]
    unsafe fn get_unchecked(
        &'a mut self,
        _world: NonNull<World>,
        _queue: &mut dyn ActionQueue,
    ) -> &'a mut ScopedAllocator {
        &mut self.allocator
    }

    #[inline]
    unsafe fn flush_unchecked(&'a mut self, _world: NonNull<World>, _queue: &mut dyn ActionQueue) {
        self.allocator.scope.reset();
    }
}

impl FnArgCache for ScopedAllocatorCache {
    #[inline]
    fn is_local(&self) -> bool {
        false
    }

    #[inline]
    fn world_access(&self) -> Option<Access> {
        None
    }

    #[inline]
    fn skips_archetype(&self, _archetype: &Archetype) -> bool {
        false
    }

    #[inline]
    fn access_component(&self, _id: TypeId) -> Option<Access> {
        None
    }

    #[inline]
    fn access_resource(&self, _id: TypeId) -> Option<Access> {
        None
    }
}

impl Default for ScopedAllocatorCache {
    #[inline]
    fn default() -> Self {
        ScopedAllocatorCache {
            allocator: ScopedAllocator {
                scope: Scope::new(),
            },
        }
    }
}
