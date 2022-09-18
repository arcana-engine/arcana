use std::fmt::{self, Display};

use edict::{
    component::Component,
    entity::EntityId,
    query::{Alt, Entities, Modified, With},
    relation::{ChildOf, FilterNotRelates, Related, RelatesExclusive, Relation},
    world::QueryRef,
};
use hashbrown::{HashMap, HashSet};

use crate::scoped_allocator::ScopedAllocator;

#[cfg(feature = "2d")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Local2 {
    pub iso: na::Isometry2<f32>,
}

#[cfg(feature = "2d")]
impl Relation for Local2 {
    const EXCLUSIVE: bool = true;
    const OWNED: bool = true;
}

#[cfg(feature = "2d")]
impl Display for Local2 {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.iso, fmt)
    }
}

#[cfg(feature = "2d")]
impl Local2 {
    #[inline]
    pub fn identity() -> Self {
        Local2 {
            iso: na::Isometry2::identity(),
        }
    }

    #[inline]
    pub fn new(iso: na::Isometry2<f32>) -> Self {
        Local2 { iso }
    }

    #[inline]
    pub fn from_translation(tr: na::Translation2<f32>) -> Self {
        Local2 {
            iso: na::Isometry2::from_parts(tr, na::UnitComplex::identity()),
        }
    }

    #[inline]
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
    #[inline]
    pub fn identity() -> Self {
        Global2 {
            iso: na::Isometry2::identity(),
        }
    }

    #[inline]
    pub fn is_identity(&self) -> bool {
        self.iso == na::Isometry2::identity()
    }

    #[inline]
    pub fn new(iso: na::Isometry2<f32>) -> Self {
        Global2 { iso }
    }

    #[inline]
    pub fn append_iso(&mut self, iso: &na::Isometry2<f32>) -> &mut Self {
        self.iso *= iso;
        self
    }

    #[inline]
    pub fn append_translation(&mut self, translation: &na::Translation2<f32>) -> &mut Self {
        self.iso *= translation;
        self
    }

    #[inline]
    pub fn append_rotation(&mut self, rot: &na::UnitComplex<f32>) -> &mut Self {
        self.iso *= rot;
        self
    }

    #[inline]
    pub fn append_local(&mut self, local: &Local2) -> &mut Self {
        self.append_iso(&local.iso)
    }

    #[inline]
    pub fn to_homogeneous(&self) -> na::Matrix4<f32> {
        self.iso.to_homogeneous().to_homogeneous()
    }

    #[inline]
    pub fn to_affine(&self) -> na::Affine2<f32> {
        na::Affine2::from_matrix_unchecked(self.iso.to_homogeneous())
    }
}

#[cfg(feature = "2d")]
impl From<na::Point2<f32>> for Global2 {
    #[inline]
    fn from(point: na::Point2<f32>) -> Self {
        Global2 {
            iso: na::Isometry2::from(point),
        }
    }
}

#[cfg(feature = "2d")]
impl From<na::Isometry2<f32>> for Global2 {
    #[inline]
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
    const OWNED: bool = true;
}

#[cfg(feature = "3d")]
impl Display for Local3 {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.iso, fmt)
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
pub fn scene_system2(
    mut roots_modified: QueryRef<
        (Entities, Modified<&Global2>, Related<ChildOf>),
        FilterNotRelates<ChildOf>,
    >,
    mut modified: QueryRef<(
        Entities,
        RelatesExclusive<&ChildOf>,
        Modified<&Local2>,
        &Global2,
    )>,
    mut children: QueryRef<Related<ChildOf>, (With<Local2>, With<Global2>)>,
    mut global: QueryRef<&Global2>,
    mut update: QueryRef<(&Local2, Alt<Global2>)>,
    scope: &mut ScopedAllocator,
) {
    use std::collections::VecDeque;

    let mut subtrees =
        HashMap::<EntityId, (EntityId, Vec<(EntityId, EntityId), _>), _, _>::new_in(&**scope);
    let mut visited = HashSet::new_in(&**scope);
    let mut to_visit = Vec::new_in(&**scope);
    let mut globals = HashMap::new_in(&**scope);

    roots_modified.for_each(|(entity, global, children)| {
        globals.insert(entity, *global);
        for &child in children {
            to_visit.push((entity, child));
        }
    });
    drop(roots_modified);

    modified.for_each(|(entity, (ChildOf, parent), _local, global)| {
        globals.insert(entity, *global);
        to_visit.push((parent, entity));
    });
    drop(modified);

    for (parent, entity) in to_visit.drain(..) {
        if !visited.insert(entity) {
            continue;
        }

        let mut subtree = Vec::new_in(&**scope);

        let mut to_visit = VecDeque::new_in(&**scope);

        if let Ok(children) = children.get_one(entity) {
            for child in children {
                to_visit.push_back((entity, *child));
            }
        }

        while let Some((parent, entity)) = to_visit.pop_front() {
            subtree.push((parent, entity));

            match subtrees.get(&entity) {
                None => {
                    let old = visited.insert(entity);
                    debug_assert!(old, "Entity visited twice");
                    if let Ok(children) = children.get_one(entity) {
                        for child in children {
                            to_visit.push_back((entity, *child));
                        }
                    }
                }
                Some((e, child_subtree)) => {
                    debug_assert!(visited.contains(&entity));
                    debug_assert_eq!(*e, entity);
                    subtree.extend_from_slice(child_subtree);
                }
            }
        }

        subtrees.insert(entity, (parent, subtree));
    }
    drop(to_visit);
    drop(visited);

    for (_, (parent, _)) in &subtrees {
        globals
            .entry(*parent)
            .or_insert_with(|| global.get_one_copied(*parent).unwrap());
    }
    drop(global);

    for (entity, (parent, subtree)) in subtrees {
        let (local, mut global) = update.get_one(entity).unwrap();
        global.iso = globals[&parent].iso * local.iso;
        globals.insert(entity, *global);

        for (parent, entity) in subtree {
            let (local, mut global) = update.get_one(entity).unwrap();
            global.iso = globals[&parent].iso * local.iso;
            globals.insert(entity, *global);
        }
    }
}

#[cfg(feature = "3d")]
pub fn scene_system3(
    mut roots_modified: QueryRef<
        (Entities, Modified<&Global3>, Related<ChildOf>),
        FilterNotRelates<ChildOf>,
    >,
    mut modified: QueryRef<(
        Entities,
        RelatesExclusive<&ChildOf>,
        Modified<&Local3>,
        &Global3,
    )>,
    mut children: QueryRef<Related<ChildOf>, (With<Local3>, With<Global3>)>,
    mut global: QueryRef<&Global3>,
    mut update: QueryRef<(&Local3, Alt<Global3>)>,
    scope: &mut ScopedAllocator,
) {
    use std::collections::VecDeque;

    let mut subtrees =
        HashMap::<EntityId, (EntityId, Vec<(EntityId, EntityId), _>), _, _>::new_in(&**scope);
    let mut visited = HashSet::new_in(&**scope);
    let mut to_visit = Vec::new_in(&**scope);
    let mut globals = HashMap::new_in(&**scope);

    roots_modified.for_each(|(entity, global, children)| {
        globals.insert(entity, *global);
        for &child in children {
            to_visit.push((entity, child));
        }
    });
    drop(roots_modified);

    modified.for_each(|(entity, (ChildOf, parent), _local, global)| {
        globals.insert(entity, *global);
        to_visit.push((parent, entity));
    });
    drop(modified);

    for (parent, entity) in to_visit.drain(..) {
        if !visited.insert(entity) {
            continue;
        }

        let mut subtree = Vec::new_in(&**scope);

        let mut to_visit = VecDeque::new_in(&**scope);

        if let Ok(children) = children.get_one(entity) {
            for child in children {
                to_visit.push_back((entity, *child));
            }
        }

        while let Some((parent, entity)) = to_visit.pop_front() {
            subtree.push((parent, entity));

            match subtrees.get(&entity) {
                None => {
                    let old = visited.insert(entity);
                    debug_assert!(old, "Entity visited twice");
                    if let Ok(children) = children.get_one(entity) {
                        for child in children {
                            to_visit.push_back((entity, *child));
                        }
                    }
                }
                Some((e, child_subtree)) => {
                    debug_assert!(visited.contains(&entity));
                    debug_assert_eq!(*e, entity);
                    subtree.extend_from_slice(child_subtree);
                }
            }
        }

        subtrees.insert(entity, (parent, subtree));
    }
    drop(to_visit);
    drop(visited);

    for (_, (parent, _)) in &subtrees {
        globals
            .entry(*parent)
            .or_insert_with(|| global.get_one_copied(*parent).unwrap());
    }
    drop(global);

    for (entity, (parent, subtree)) in subtrees {
        let (local, mut global) = update.get_one(entity).unwrap();
        global.iso = globals[&parent].iso * local.iso;
        globals.insert(entity, *global);

        for (parent, entity) in subtree {
            let (local, mut global) = update.get_one(entity).unwrap();
            global.iso = globals[&parent].iso * local.iso;
            globals.insert(entity, *global);
        }
    }
}
