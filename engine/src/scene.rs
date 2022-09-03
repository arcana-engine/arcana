use std::fmt::{self, Display};

use edict::{
    prelude::{ActionEncoder, Component, EntityId},
    query::{Alt, Modified, With},
    relation::{ChildOf, FilterNotRelates, FilterRelates, Related, RelatesExclusive, Relation},
    world::QueryRef,
};

use crate::scoped_allocator::ScopedAllocator;

#[cfg(feature = "2d")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Local2 {
    pub iso: na::Isometry2<f32>,
}

#[cfg(feature = "2d")]
impl Relation for Local2 {
    const EXCLUSIVE: bool = true;

    fn on_target_drop(entity: EntityId, _target: EntityId, encoder: &mut ActionEncoder) {
        encoder.despawn(entity);
    }
}

#[cfg(feature = "2d")]
impl Display for Local2 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}@{}", self.parent, self.iso)
    }
}

#[cfg(feature = "2d")]
impl Local2 {
    pub fn identity() -> Self {
        Local2 {
            iso: na::Isometry2::identity(),
        }
    }

    pub fn new(iso: na::Isometry2<f32>) -> Self {
        Local2 { iso }
    }

    pub fn from_translation(tr: na::Translation2<f32>) -> Self {
        Local2 {
            iso: na::Isometry2::from_parts(tr, na::UnitComplex::identity()),
        }
    }

    pub fn from_rotation(rot: na::UnitComplex<f32>) -> Self {
        Local2 {
            iso: na::Isometry2::from_parts(na::Translation2::identity(), rot),
        }
    }
}

#[cfg(feature = "2d")]
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Component)]
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

    pub fn to_affine(&self) -> na::Affine2<f32> {
        na::Affine2::from_matrix_unchecked(self.iso.to_homogeneous())
    }
}

#[cfg(feature = "2d")]
impl From<na::Point2<f32>> for Global2 {
    fn from(point: na::Point2<f32>) -> Self {
        Global2 {
            iso: na::Isometry2::from(point),
        }
    }
}

#[cfg(feature = "2d")]
impl From<na::Isometry2<f32>> for Global2 {
    fn from(iso: na::Isometry2<f32>) -> Self {
        Global2 { iso }
    }
}

#[cfg(feature = "3d")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Local3 {
    pub iso: na::Isometry3<f32>,
}

#[cfg(feature = "3d")]
impl Relation for Local3 {
    const EXCLUSIVE: bool = true;

    fn on_target_drop(entity: EntityId, _target: EntityId, encoder: &mut ActionEncoder) {
        encoder.despawn(entity);
    }
}

#[cfg(feature = "3d")]
impl Display for Local3 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}@{}", self.parent, self.iso)
    }
}

#[cfg(feature = "3d")]
impl Local3 {
    pub fn identity() -> Self {
        Local3 {
            iso: na::Isometry3::identity(),
        }
    }

    pub fn new(iso: na::Isometry3<f32>) -> Self {
        Local3 { iso }
    }

    pub fn from_translation(tr: na::Translation3<f32>) -> Self {
        Local3 {
            iso: na::Isometry3::from_parts(tr, na::UnitQuaternion::identity()),
        }
    }

    pub fn from_rotation(rot: na::UnitQuaternion<f32>) -> Self {
        Local3 {
            iso: na::Isometry3::from_parts(na::Translation3::identity(), rot),
        }
    }
}

#[cfg(feature = "3d")]
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Component)]
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

    pub fn to_affine(&self) -> na::Affine3<f32> {
        na::Affine3::from_matrix_unchecked(self.iso.to_homogeneous())
    }
}

#[cfg(feature = "3d")]
impl From<na::Point3<f32>> for Global3 {
    fn from(point: na::Point3<f32>) -> Self {
        Global3 {
            iso: na::Isometry3::from(point),
        }
    }
}

#[cfg(feature = "3d")]
impl From<na::Isometry3<f32>> for Global3 {
    fn from(iso: na::Isometry3<f32>) -> Self {
        Global3 { iso }
    }
}

#[cfg(feature = "2d")]
fn scene_system2(
    roots_modified: QueryRef<(Modified<&Global2>, Related<ChildOf>), FilterNotRelates<ChildOf>>,
    modified: QueryRef<Modified<&Local2>, (FilterRelates<ChildOf>, With<Global2>)>,
    children: QueryRef<Related<ChildOf>, (With<Local2>, With<Global2>)>,
    update: QueryRef<(RelatesExclusive<&ChildOf>, &Local2, Alt<Global2>)>,
    read_global: QueryRef<&Global2>,
    scope: &mut ScopedAllocator,
) {
    use hashbrown::{hash_map::Entry, HashMap};

    let mut to_update = Vec::new_in(&**scope);
    let mut visiting_counter = HashMap::new_in(&**scope);

    roots_modified.for_each(|(global, children)| {
        for &child in children {
            *visiting_counter.entry(child).or_insert(0) += 1;
            to_update.push(child);
        }
    });

    let mut i = 0;
    while i < to_update.len() {
        let entity = to_update[i];

        if let Ok(children) = children.one(entity) {
            for &child in children {
                let counter = visiting_counter.entry(child).or_insert(0);
                *counter += 1;
                debug_assert_eq!(*counter, 1, "Two roots in hierarchy");
                to_update.push(child);
            }
        }

        i += 1;
    }

    for (entity, _local) in modified {
        if visiting_counter.contains_key(&entity) {
            continue;
        }

        match visiting_counter.entry(entity) {
            Entry::Occupied(_) => continue,
            Entry::Vacant(entry) => {
                entry.insert(1);
                to_update.push(entity);
            }
        }
    }

    while i < to_update.len() {
        let entity = to_update[i];

        if let Ok(children) = children.one(entity) {
            for &child in children {
                *visiting_counter.entry(child).or_insert(0) += 1;
                to_update.push(child);
            }
        }

        i += 1;
    }

    for entity in to_update {
        let counter = &mut visiting_counter[&entity];
        *counter -= 1;

        if *counter == 0 {
            let ((_, parent), local, global) = update.one(entity).unwrap();

            let parent_global = read_global.one(parent).unwrap();
            global.iso = parent_global.iso * local.iso;
        }
    }
}

#[cfg(feature = "3d")]
fn scene_system3(
    roots_modified: QueryRef<(Modified<&Global3>, Related<ChildOf>), FilterNotRelates<ChildOf>>,
    modified: QueryRef<Modified<&Local3>, (FilterRelates<ChildOf>, With<Global3>)>,
    children: QueryRef<Related<ChildOf>, (With<Local3>, With<Global3>)>,
    update: QueryRef<(RelatesExclusive<&ChildOf>, &Local3, Alt<Global3>)>,
    read_global: QueryRef<&Global3>,
    scope: &mut ScopedAllocator,
) {
    use hashbrown::{hash_map::Entry, HashMap};

    let mut to_update = Vec::new_in(&**scope);
    let mut visiting_counter = HashMap::new_in(&**scope);

    roots_modified.for_each(|(global, children)| {
        for &child in children {
            *visiting_counter.entry(child).or_insert(0) += 1;
            to_update.push(child);
        }
    });

    let mut i = 0;
    while i < to_update.len() {
        let entity = to_update[i];

        if let Ok(children) = children.one(entity) {
            for &child in children {
                let counter = visiting_counter.entry(child).or_insert(0);
                *counter += 1;
                debug_assert_eq!(*counter, 1, "Two roots in hierarchy");
                to_update.push(child);
            }
        }

        i += 1;
    }

    for (entity, _local) in modified {
        if visiting_counter.contains_key(&entity) {
            continue;
        }

        match visiting_counter.entry(entity) {
            Entry::Occupied(_) => continue,
            Entry::Vacant(entry) => {
                entry.insert(1);
                to_update.push(entity);
            }
        }
    }

    while i < to_update.len() {
        let entity = to_update[i];

        if let Ok(children) = children.one(entity) {
            for &child in children {
                *visiting_counter.entry(child).or_insert(0) += 1;
                to_update.push(child);
            }
        }

        i += 1;
    }

    for entity in to_update {
        let counter = &mut visiting_counter[&entity];
        *counter -= 1;

        if *counter == 0 {
            let ((_, parent), local, global) = update.one(entity).unwrap();

            let parent_global = read_global.one(parent).unwrap();
            global.iso = parent_global.iso * local.iso;
        }
    }
}
