use hecs::{Entity, World};

use crate::{Res, Spawner};

/// Prefabs are predefiend sets of components that can be spawned onto the World.
/// Prefabs serve as higher-level composition blocks of the World.
/// In networked simulation, upone receiving new entity with prefab component it will be spawned.
pub trait Prefab {
    /// Spawns prefab instance into the World.
    /// Prefab spawn *must* create new entity.
    /// Prefab *should* insert themselves as one of the components to that entity.
    /// Prefab *can* create additional entities.
    /// Additional entities *should* be child entities if they are part of the scenegraph.
    /// Spawning can also spawn tasks to finish entity creation asynchronously.
    /// This function *must* return main spawned entity index.
    fn spawn(self, res: &mut Res, world: &mut World, spawner: &Spawner) -> Entity;
}
