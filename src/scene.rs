use std::fmt::{self, Display};

use hashbrown::HashSet;
use hecs::{Entity, EntityRef, World};
use scoped_arena::Scope;

use crate::{
    debug::EntityRefDisplay as _,
    system::{System, SystemContext},
};

#[cfg(feature = "2d")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Local2 {
    pub parent: Entity,
    pub iso: na::Isometry2<f32>,
}

#[cfg(feature = "2d")]
impl Display for Local2 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}@{}", self.parent.to_bits(), self.iso)
    }
}

#[cfg(feature = "2d")]
impl Local2 {
    pub fn identity(parent: Entity) -> Self {
        Local2 {
            parent,
            iso: na::Isometry2::identity(),
        }
    }

    pub fn new(parent: Entity, iso: na::Isometry2<f32>) -> Self {
        Local2 { parent, iso }
    }

    pub fn from_translation(parent: Entity, tr: na::Translation2<f32>) -> Self {
        Local2 {
            parent,
            iso: na::Isometry2::from_parts(tr, na::UnitComplex::identity()),
        }
    }

    pub fn from_rotation(parent: Entity, rot: na::UnitComplex<f32>) -> Self {
        Local2 {
            parent,
            iso: na::Isometry2::from_parts(na::Translation2::identity(), rot),
        }
    }
}

#[cfg(feature = "2d")]
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Global2 {
    pub iso: na::Isometry2<f32>,
}

#[cfg(feature = "2d")]
impl Default for Global2 {
    fn default() -> Self {
        Global2::identity()
    }
}

#[cfg(feature = "2d")]
impl Display for Global2 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.iso, fmt)
    }
}

#[cfg(feature = "2d")]
impl Global2 {
    pub fn identity() -> Self {
        Global2 {
            iso: na::Isometry2::identity(),
        }
    }

    pub fn is_identity(&self) -> bool {
        self.iso == na::Isometry2::identity()
    }

    pub fn new(iso: na::Isometry2<f32>) -> Self {
        Global2 { iso }
    }

    pub fn append_iso(&mut self, iso: &na::Isometry2<f32>) -> &mut Self {
        self.iso *= iso;
        self
    }

    pub fn append_translation(&mut self, translation: &na::Translation2<f32>) -> &mut Self {
        self.iso *= translation;
        self
    }

    pub fn append_rotation(&mut self, rot: &na::UnitComplex<f32>) -> &mut Self {
        self.iso *= rot;
        self
    }

    pub fn append_local(&mut self, local: &Local2) -> &mut Self {
        self.append_iso(&local.iso)
    }

    pub fn to_homogeneous(&self) -> na::Matrix4<f32> {
        self.iso.to_homogeneous().to_homogeneous()
    }
}

#[cfg(feature = "3d")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Local3 {
    pub parent: Entity,
    pub iso: na::Isometry3<f32>,
}

#[cfg(feature = "3d")]
impl Display for Local3 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}@{}", self.parent.to_bits(), self.iso)
    }
}

#[cfg(feature = "3d")]
impl Local3 {
    pub fn identity(parent: Entity) -> Self {
        Local3 {
            parent,
            iso: na::Isometry3::identity(),
        }
    }

    pub fn new(parent: Entity, iso: na::Isometry3<f32>) -> Self {
        Local3 { parent, iso }
    }

    pub fn from_translation(parent: Entity, tr: na::Translation3<f32>) -> Self {
        Local3 {
            parent,
            iso: na::Isometry3::from_parts(tr, na::UnitQuaternion::identity()),
        }
    }

    pub fn from_rotation(parent: Entity, rot: na::UnitQuaternion<f32>) -> Self {
        Local3 {
            parent,
            iso: na::Isometry3::from_parts(na::Translation3::identity(), rot),
        }
    }
}

#[cfg(feature = "3d")]
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Global3 {
    pub iso: na::Isometry3<f32>,
}

#[cfg(feature = "3d")]
impl Default for Global3 {
    fn default() -> Self {
        Global3::identity()
    }
}

#[cfg(feature = "3d")]
impl Display for Global3 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.iso, fmt)
    }
}

#[cfg(feature = "3d")]
impl Global3 {
    pub fn identity() -> Self {
        Global3 {
            iso: na::Isometry3::identity(),
        }
    }

    pub fn is_identity(&self) -> bool {
        self.iso == na::Isometry3::identity()
    }

    pub fn new(iso: na::Isometry3<f32>) -> Self {
        Global3 { iso }
    }

    pub fn append_iso(&mut self, iso: &na::Isometry3<f32>) -> &mut Self {
        self.iso *= iso;
        self
    }

    pub fn append_translation(&mut self, translation: &na::Translation3<f32>) -> &mut Self {
        self.iso *= translation;
        self
    }

    pub fn append_rotation(&mut self, rot: &na::UnitQuaternion<f32>) -> &mut Self {
        self.iso *= rot;
        self
    }

    pub fn append_local(&mut self, local: &Local3) -> &mut Self {
        self.append_iso(&local.iso)
    }

    pub fn to_homogeneous(&self) -> na::Matrix4<f32> {
        self.iso.to_homogeneous()
    }
}

pub struct SceneSystem;

impl System for SceneSystem {
    fn name(&self) -> &str {
        "Scene"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let mut despawn = Vec::new_in(&*cx.scope);

        #[cfg(feature = "2d")]
        let mut updated = HashSet::new_in(&*cx.scope);

        #[cfg(feature = "2d")]
        for (entity, local) in cx.world.query::<&Local2>().with::<Global2>().iter() {
            if updated.insert(entity.id()) {
                update_global_2(
                    entity,
                    cx.world.entity(entity).unwrap(),
                    local,
                    cx.world,
                    &*cx.scope,
                    &mut updated,
                    &mut despawn,
                );
            }
        }

        #[cfg(feature = "3d")]
        let mut updated = HashSet::new_in(&*cx.scope);

        #[cfg(feature = "3d")]
        for (entity, local) in cx.world.query::<&Local3>().with::<Global3>().iter() {
            if updated.insert(entity.id()) {
                update_global_3(
                    entity,
                    cx.world.entity(entity).unwrap(),
                    local,
                    cx.world,
                    &*cx.scope,
                    &mut updated,
                    &mut despawn,
                );
            }
        }

        // Despawn entities whose parents are despawned.
        for entity in despawn {
            let _ = cx.world.despawn(entity);
        }

        Ok(())
    }
}

#[cfg(feature = "2d")]
fn update_global_2<'a, 'b>(
    entity: Entity,
    entity_ref: EntityRef<'a>,
    local: &Local2,
    world: &'a World,
    scope: &'b Scope<'_>,
    updated: &mut HashSet<u32, ahash::RandomState, &'b Scope<'_>>,
    despawn: &mut Vec<Entity, &'b Scope<'_>>,
) -> Option<hecs::RefMut<'a, Global2>> {
    let parent_ref = match world.entity(local.parent) {
        Ok(parent_ref) => parent_ref,
        Err(hecs::NoSuchEntity) => {
            despawn.push(entity);
            return None;
        }
    };
    let parent_local = parent_ref.get::<Local2>();

    match parent_local {
        None => {
            // Parent is root node.
            match parent_ref.get::<Global2>() {
                Some(parent_global_ref) => {
                    // Parent is root node.
                    let mut global = Global2::clone(&*parent_global_ref);
                    drop(parent_global_ref);
                    global.append_local(local);
                    let mut global_ref = entity_ref.get_mut::<Global2>().unwrap();
                    *global_ref = global;

                    Some(global_ref)
                }
                None => {
                    // Parent is not in hierarchy.
                    tracing::warn!(
                        "Entity's ({}) parent is not in scene and shall be despawned",
                        entity_ref.display(entity)
                    );
                    despawn.push(entity);
                    None
                }
            }
        }
        Some(parent_local) => {
            let parent_global = if updated.insert(local.parent.id()) {
                update_global_2(
                    local.parent,
                    parent_ref,
                    &parent_local,
                    world,
                    scope,
                    updated,
                    despawn,
                )
            } else {
                parent_ref.get_mut::<Global2>()
            };

            match parent_global {
                Some(parent_global) => {
                    let mut global = Global2::clone(&*parent_global);
                    drop(parent_global);
                    global.append_local(local);
                    let mut global_ref = entity_ref.get_mut::<Global2>().unwrap();
                    *global_ref = global;
                    Some(global_ref)
                }
                None => {
                    despawn.push(entity);
                    None
                }
            }
        }
    }
}

#[cfg(feature = "3d")]
fn update_global_3<'a, 'b>(
    entity: Entity,
    entity_ref: EntityRef<'a>,
    local: &Local3,
    world: &'a World,
    scope: &'b Scope<'_>,
    updated: &mut HashSet<u32, ahash::RandomState, &'b Scope<'_>>,
    despawn: &mut Vec<Entity, &'b Scope<'_>>,
) -> Option<hecs::RefMut<'a, Global3>> {
    let parent_ref = match world.entity(local.parent) {
        Ok(parent_ref) => parent_ref,
        Err(hecs::NoSuchEntity) => {
            despawn.push(entity);
            return None;
        }
    };
    let parent_local = parent_ref.get::<Local3>();

    match parent_local {
        None => {
            // Parent has no parent node.
            match parent_ref.get::<Global3>() {
                Some(parent_global_ref) => {
                    // Parent is root node.
                    let mut global = Global3::clone(&*parent_global_ref);
                    drop(parent_global_ref);
                    global.append_local(local);
                    let mut global_ref = entity_ref.get_mut::<Global3>().unwrap();
                    *global_ref = global;

                    Some(global_ref)
                }
                None => {
                    // Parent is not in hierarchy.
                    tracing::warn!(
                        "Entity's ({}) parent is not in scene and shall be despawned",
                        entity_ref.display(entity)
                    );
                    despawn.push(entity);
                    None
                }
            }
        }
        Some(parent_local) => {
            let parent_global = if updated.insert(local.parent.id()) {
                update_global_3(
                    local.parent,
                    parent_ref,
                    &parent_local,
                    world,
                    scope,
                    updated,
                    despawn,
                )
            } else {
                parent_ref.get_mut::<Global3>()
            };

            match parent_global {
                Some(parent_global) => {
                    let mut global = Global3::clone(&*parent_global);
                    drop(parent_global);
                    global.append_local(local);
                    let mut global_ref = entity_ref.get_mut::<Global3>().unwrap();
                    *global_ref = global;
                    Some(global_ref)
                }
                None => {
                    despawn.push(entity);
                    None
                }
            }
        }
    }
}
