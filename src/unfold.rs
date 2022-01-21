use std::{any::type_name, borrow::Borrow, marker::PhantomData, mem::MaybeUninit};

use goods::{Asset, AssetBuild, AssetId, Loader, TypedAssetId};
use hecs::{Component, Entity, Query, World};

use crate::{
    assets::AssetLoadCache,
    system::{System, SystemContext},
};

pub use arcana_proc::Unfold;

/// Unfold is a component type that triggers spawning of other components and/or additional entities.
/// This trait is typically used as final step of game state deserialization.
/// Serializable unfolds are used to sync complex game state using fewer items.
///
/// Unfold is associated with [`UnfoldSystem`] that perform those tasks.
///
/// This trait implementation is often boilerplate-y, so derive macro is provided.
/// Take not however, that deriving this trait without any associated attributes produces
/// system that does exactly nothing.
///
/// Attributes of the form `#[unfold(...)]` augment generated system.
///
/// `#[unfold(asset)]` placed over field with type `TypedAssetId<A>` or
/// `#[unfold(asset: AssetType)] over field with type `AssetId` will cause unfold system
/// to load specified asset and put it as a component to the same entity, and then keep it sync in case of id changes, or unfold component is removed
///
/// Warning: unfold system will not have any chance to see if asset component changes.
///
/// If added component is a unfold, this will create cascade effect.
///
/// `#[unfold(funcname)]` placed over unfold type will cause `funcname` to be called.
/// That function should take entity id, this type and asset references for each field with `#[unfold(asset)]` attribute as arguments and return `Unfold` structure with bundle `insert` and iterator of bundles `spawn`.
/// Bundle `insert` is then added to the entity.
/// For each bundle item in `spawn` an entity will be spawned with that bundle.
/// Spawned entities should be able to despawn themselves after unfold entity despawn, for example having `Local2/3` component would do the trick.
/// On each change of the unfold, previously spawned entities are despawned by unfold system and new ones are spawned.
///
/// `#[unfold(funcname)]` on type suppresses default behavior attributes on individual fields. That is, components won't be added for fields with `#[unfold(asset)]` attribute.
///
pub trait Unfold {
    /// System that operates on unfold.
    ///
    /// Typical unfold system ensures that additional components and entities are spawned and are in sync.
    /// For example, unfold system may load an asset using id from unfold component
    /// and put loaded asset value as component next to the unfold component.
    /// If id changes in the unfold component, that system will replace asset component automatically.\
    /// Unfold systems are not limited to this behavior.
    type UnfoldSystem: System;
}

pub struct UnfoldResult<T, I> {
    pub insert: T,
    pub spawn: I,
}

trait TupleComponentsRemove {
    fn remove(world: &mut World, entity: Entity);
}

impl TupleComponentsRemove for () {
    fn remove(world: &mut World, entity: Entity) {}
}

macro_rules! impl_tuple {
    () => {};
}
impl<A> TupleComponentsRemove for (A,)
where
    A: Component,
{
    fn remove(world: &mut World, entity: Entity) {
        let _ = world.remove_one::<A>(e);
    }
}

/// Dummy system for `Unfold`s that need no actions to be performed.
pub struct DummyUnfoldSystem;

impl System for DummyUnfoldSystem {
    fn name(&self) -> &str {
        "Dummy unfold system"
    }

    fn run(&mut self, _: SystemContext<'_>) -> eyre::Result<()> {
        Ok(())
    }
}