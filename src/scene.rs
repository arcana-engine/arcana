use std::{
    collections::VecDeque,
    fmt::{self, Display},
};

use bitsetium::{BitEmpty, BitSet, BitTest, Bits65536};
use edict::{prelude::EntityId, world::EntityError};

use crate::system::{System, SystemContext};

#[cfg(feature = "2d")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Local2 {
    pub parent: EntityId,
    pub iso: na::Isometry2<f32>,
}

#[cfg(feature = "2d")]
impl Display for Local2 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}@{}", self.parent, self.iso)
    }
}

#[cfg(feature = "2d")]
impl Local2 {
    pub fn identity(parent: EntityId) -> Self {
        Local2 {
            parent,
            iso: na::Isometry2::identity(),
        }
    }

    pub fn new(parent: EntityId, iso: na::Isometry2<f32>) -> Self {
        Local2 { parent, iso }
    }

    pub fn from_translation(parent: EntityId, tr: na::Translation2<f32>) -> Self {
        Local2 {
            parent,
            iso: na::Isometry2::from_parts(tr, na::UnitComplex::identity()),
        }
    }

    pub fn from_rotation(parent: EntityId, rot: na::UnitComplex<f32>) -> Self {
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
    pub parent: EntityId,
    pub iso: na::Isometry3<f32>,
}

#[cfg(feature = "3d")]
impl Display for Local3 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}@{}", self.parent, self.iso)
    }
}

#[cfg(feature = "3d")]
impl Local3 {
    pub fn identity(parent: EntityId) -> Self {
        Local3 {
            parent,
            iso: na::Isometry3::identity(),
        }
    }

    pub fn new(parent: EntityId, iso: na::Isometry3<f32>) -> Self {
        Local3 { parent, iso }
    }

    pub fn from_translation(parent: EntityId, tr: na::Translation3<f32>) -> Self {
        Local3 {
            parent,
            iso: na::Isometry3::from_parts(tr, na::UnitQuaternion::identity()),
        }
    }

    pub fn from_rotation(parent: EntityId, rot: na::UnitQuaternion<f32>) -> Self {
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

pub struct SceneSystem {
    #[cfg(feature = "2d")]
    cap_2: usize,
    #[cfg(feature = "3d")]
    cap_3: usize,
}

impl SceneSystem {
    pub const fn new() -> Self {
        SceneSystem {
            #[cfg(feature = "2d")]
            cap_2: 0,
            #[cfg(feature = "3d")]
            cap_3: 0,
        }
    }
}

impl System for SceneSystem {
    fn name(&self) -> &str {
        "Scene"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        #[cfg(feature = "2d")]
        {
            let mut update = VecDeque::with_capacity_in(self.cap_2, &*cx.scope);
            let mut ready = Bits65536::empty();

            let mut count_2 = 0;

            let query = cx.world.query_mut::<&Local2>().with::<Global2>();

            for (entity, local) in query {
                update.push_back((entity, *local));
            }

            while let Some((entity, local)) = update.front() {
                if !ready.test(entity.bits() as usize) {
                    ready.set(entity.bits() as usize);
                    count_2 += 1;
                    match cx
                        .world
                        .query_one_mut::<(Option<&Local2>, &Global2)>(&local.parent)
                    {
                        Ok((None, parent_global)) => {
                            let iso = parent_global.iso * local.iso;
                            cx.world.query_one_mut::<&mut Global2>(entity).unwrap().iso = iso;
                            update.pop_front();
                        }
                        Ok((Some(parent_local), parent_global)) => {
                            if !ready.test(local.parent.bits() as usize) {
                                ready.set(local.parent.bits() as usize);
                                let elem = (local.parent, *parent_local);
                                update.push_front(elem);
                            } else {
                                let iso = parent_global.iso * local.iso;
                                cx.world.query_one_mut::<&mut Global2>(entity).unwrap().iso = iso;
                                update.pop_front();
                            }
                        }
                        Err(EntityError::NoSuchEntity) => {
                            let _ = cx.world.despawn(entity);
                            update.pop_front();
                        }
                        Err(EntityError::MissingComponents) => {
                            let entity = *entity;
                            let _ = cx.world.remove::<Global2>(&entity);
                            update.pop_front();
                        }
                    }
                }
            }

            if count_2 > self.cap_2 {
                self.cap_2 = count_2;
            } else {
                self.cap_2 = self.cap_2 / 2 + count_2 / 2;
            }
        }

        #[cfg(feature = "3d")]
        {
            let mut update = VecDeque::with_capacity_in(self.cap_3, &*cx.scope);
            let mut ready = Bits65536::empty();

            let mut count_3 = 0;

            let query = cx.world.query_mut::<&Local3>().with::<Global3>();

            for (entity, local) in query {
                update.push_back((entity, *local));
            }

            while let Some((entity, local)) = update.front() {
                if !ready.test(entity.bits() as usize) {
                    ready.set(entity.bits() as usize);
                    count_3 += 1;
                    match cx
                        .world
                        .query_one_mut::<(Option<&Local3>, &Global3)>(&local.parent)
                    {
                        Ok((None, parent_global)) => {
                            let iso = parent_global.iso * local.iso;
                            cx.world.query_one_mut::<&mut Global3>(entity).unwrap().iso = iso;
                            update.pop_front();
                        }
                        Ok((Some(parent_local), parent_global)) => {
                            if !ready.test(local.parent.bits() as usize) {
                                ready.set(local.parent.bits() as usize);
                                let elem = (local.parent, *parent_local);
                                update.push_front(elem);
                            } else {
                                let iso = parent_global.iso * local.iso;
                                cx.world.query_one_mut::<&mut Global3>(entity).unwrap().iso = iso;
                                update.pop_front();
                            }
                        }
                        Err(EntityError::NoSuchEntity) => {
                            let _ = cx.world.despawn(entity);
                            update.pop_front();
                        }
                        Err(EntityError::MissingComponents) => {
                            let _ = cx.world.remove::<Global3>(entity);
                            update.pop_front();
                        }
                    }
                }
            }

            if count_3 > self.cap_3 {
                self.cap_3 = count_3;
            } else {
                self.cap_3 = self.cap_3 / 2 + count_3 / 2;
            }
        }
    }
}
