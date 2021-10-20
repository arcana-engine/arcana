use hecs::{Component, Entity, World};

use crate::Res;

/// Prefabs are predefined sets of components that can be spawned onto the World.
/// Prefabs serve as higher-level composition blocks of the World.
/// In networked simulation, upon receiving new entity with prefab component it will be spawned.
pub trait Prefab {
    /// Spawns prefab instance into the World.
    /// Prefab spawn *must* create new entity.
    /// Prefab *should* insert themselves as one of the components to that entity.
    /// Prefab *can* create additional entities.
    /// Additional entities *should* be child entities if they are part of the scene-graph.
    /// Spawning can also spawn tasks to finish entity creation asynchronously.
    /// This function *must* return main spawned entity index.
    fn spawn(self, res: &mut Res, world: &mut World) -> Entity;
}

// /// Prefab component is a component that can add components and spawn entities when inserted.
// pub trait PrefabComponent: Component {
//     /// Method that should be called before component is inserted.
//     /// `entity` - the entity where component will be inserted.
//     fn pre_insert(
//         &mut self,
//         entity: Entity,
//         world: &mut World,
//         res: &mut Res,
//         spawner: &mut Spawner,
//     );
// }
