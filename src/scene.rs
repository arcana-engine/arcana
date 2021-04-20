use {
    crate::{
        bitset::BumpBitSet,
        debug::EntityRefDisplay as _,
        system::{System, SystemContext},
    },
    bumpalo::{collections::Vec as BVec, Bump},
    hecs::{Entity, EntityRef, World},
    std::fmt::{self, Display},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Local3 {
    pub parent: Entity,
    pub iso: na::Isometry3<f32>,
}

impl Display for Local3 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}@{}", self.parent.to_bits(), self.iso)
    }
}

impl Local3 {
    pub fn identity(parent: Entity) -> Self {
        Local3 {
            parent,
            iso: na::Isometry3::identity(),
        }
    }

    pub fn from_iso(parent: Entity, iso: na::Isometry3<f32>) -> Self {
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
            iso: na::Isometry3::from_parts(na::Translation3::new(0., 0., 0.), rot),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Global3 {
    pub iso: na::Isometry3<f32>,
}

impl Default for Global3 {
    fn default() -> Self {
        Global3::identity()
    }
}

impl Display for Global3 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.iso, fmt)
    }
}

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

    pub fn append_iso(&self, iso: &na::Isometry3<f32>) -> Self {
        Global3 {
            iso: self.iso * iso,
        }
    }

    pub fn append_local(&self, local: &Local3) -> Self {
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
        let mut updated = BumpBitSet::new();
        let mut despawn = BVec::new_in(cx.bump);

        for (entity, local) in cx.world.query::<&Local3>().with::<Global3>().iter() {
            update_global(
                entity,
                cx.world.entity(entity).unwrap(),
                local,
                cx.world,
                cx.bump,
                &mut updated,
                &mut despawn,
            );
        }

        // Despawn entities whose parents are despawned.
        for entity in despawn {
            let _ = cx.world.despawn(entity);
        }

        Ok(())
    }
}

fn update_global<'a>(
    entity: Entity,
    entity_ref: EntityRef<'a>,
    local: &Local3,
    world: &'a World,
    bump: &'a Bump,
    updated: &mut BumpBitSet<'a>,
    despawn: &mut BVec<'a, Entity>,
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
                    let global = parent_global_ref.append_local(local);
                    drop(parent_global_ref);

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
            let parent_global = if !updated.set(local.parent.id(), bump) {
                update_global(
                    local.parent,
                    parent_ref,
                    &parent_local,
                    world,
                    bump,
                    updated,
                    despawn,
                )
            } else {
                parent_ref.get_mut::<Global3>()
            };

            match parent_global {
                Some(parent_global) => {
                    let global = parent_global.append_local(local);
                    drop(parent_global);

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
