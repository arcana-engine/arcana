use crate::system::System;

/// Preafab is a component type that triggers spawning of other components and/or additional enities.
/// Serializable prefabs are used to sync complex game state using fewer items.
///
/// Prefab is associated with [`PrefabSystem`] that perform those tasks.
pub trait Prefab {
    /// System that operates on prefab.
    ///
    /// Typical prefab system ensures that additional components and entities are spawned and are in sync.
    /// For example, prefab system may load an asset using id from prefab component
    /// and put loaded asset value as component next to the prefab component.
    /// If id changes in the prefab component, that system will replace asset component automatically.
    ///
    /// Prefab systems are not limited to this behavior.
    type PrefabSystem: System;
}
