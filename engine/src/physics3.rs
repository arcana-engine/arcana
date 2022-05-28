use approx::relative_ne;
use arcana_time::TimeSpan;
use edict::entity::EntityId;
use flume::{unbounded, Sender};
use rapier3d::{
    dynamics::{
        CCDSolver, IntegrationParameters, IslandManager, JointSet, RigidBodyHandle, RigidBodySet,
    },
    geometry::{
        BroadPhase, ColliderHandle, ColliderSet, ContactEvent, ContactPair, IntersectionEvent,
        NarrowPhase,
    },
    na,
    pipeline::{EventHandler, PhysicsPipeline, QueryPipeline},
    prelude::{Collider, RigidBody},
};

use crate::{
    scene::Global3,
    system::{System, SystemContext, DEFAULT_TICK_SPAN},
};

pub use {parry3d::*, rapier3d::*};

// use crate::{
//     clocks::TimeSpan,
//     scene::Global3,
//     system::{System, SystemContext, DEFAULT_TICK_SPAN},
// };

pub struct ContactQueue3 {
    contacts_started: Vec<ColliderHandle>,
    contacts_stopped: Vec<ColliderHandle>,
}

impl ContactQueue3 {
    pub const fn new() -> Self {
        ContactQueue3 {
            contacts_started: Vec::new(),
            contacts_stopped: Vec::new(),
        }
    }

    pub fn drain_contacts_started(&mut self) -> std::vec::Drain<'_, ColliderHandle> {
        self.contacts_started.drain(..)
    }

    pub fn drain_contacts_stopped(&mut self) -> std::vec::Drain<'_, ColliderHandle> {
        self.contacts_stopped.drain(..)
    }
}

pub struct IntersectionQueue3 {
    intersecting_started: Vec<ColliderHandle>,
    intersecting_stopped: Vec<ColliderHandle>,
}

impl IntersectionQueue3 {
    pub const fn new() -> Self {
        IntersectionQueue3 {
            intersecting_started: Vec::new(),
            intersecting_stopped: Vec::new(),
        }
    }

    pub fn drain_intersecting_started(&mut self) -> std::vec::Drain<'_, ColliderHandle> {
        self.intersecting_started.drain(..)
    }

    pub fn drain_intersecting_stopped(&mut self) -> std::vec::Drain<'_, ColliderHandle> {
        self.intersecting_stopped.drain(..)
    }
}

pub struct Physics3 {
    pipeline: PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    ccd_solver: CCDSolver,
}

pub struct PhysicsData3 {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub islands: IslandManager,
    pub joints: JointSet,
    pub query_pipeline: QueryPipeline,
    pub gravity: na::Vector3<f32>,
}

impl PhysicsData3 {
    pub fn new() -> Self {
        PhysicsData3 {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            islands: IslandManager::new(),
            joints: JointSet::new(),
            query_pipeline: QueryPipeline::new(),
            gravity: na::Vector3::default(),
        }
    }

    pub fn body_user_data(&self, handle: RigidBodyHandle) -> Option<BodyUserData3> {
        let body = self.bodies.get(handle)?;
        BodyUserData3::get(body)
    }

    pub fn collider_user_data(&self, handle: ColliderHandle) -> Option<ColliderUserData3> {
        let collider = self.colliders.get(handle)?;
        ColliderUserData3::get(collider)
    }
}

impl Default for PhysicsData3 {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Physics3 {
    #[inline]
    pub fn new() -> Self {
        Physics3::with_tick_span(DEFAULT_TICK_SPAN)
    }

    #[inline]
    pub fn with_tick_span(tick_span: TimeSpan) -> Self {
        Physics3 {
            pipeline: PhysicsPipeline::new(),
            integration_parameters: IntegrationParameters {
                dt: tick_span.as_secs_f32(),
                ..IntegrationParameters::default()
            },
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
        }
    }
}

impl System for Physics3 {
    #[inline]
    fn name(&self) -> &str {
        "Physics"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        let data = cx.res.with(PhysicsData3::new);

        let mut remove_bodies = Vec::with_capacity_in(data.bodies.len(), &*cx.scope);
        let world = &mut *cx.world;
        data.bodies.iter().for_each(|(handle, body)| {
            if let Some(body_data) = BodyUserData3::from_user_data(body.user_data) {
                match world.query_one_mut::<&RigidBodyHandle>(&body_data.entity) {
                    Ok(body) if *body == handle => {}
                    _ => remove_bodies.push(handle),
                }
            }
        });
        for handle in remove_bodies {
            data.bodies.remove(
                handle,
                &mut data.islands,
                &mut data.colliders,
                &mut data.joints,
            );
        }

        for (entity, body) in cx.world.query_mut::<&RigidBodyHandle>() {
            let body = data.bodies.get_mut(*body).unwrap();

            match BodyUserData3::get(body) {
                Some(body_data) if body_data.entity == entity => {}
                _ => {
                    BodyUserData3 { entity }.set_to(body);

                    for (index, &collider) in body.colliders().iter().enumerate() {
                        let collider = data.colliders.get_mut(collider).unwrap();
                        ColliderUserData3 {
                            entity,
                            body_index: index,
                        }
                        .set_to(collider);
                    }
                }
            }
        }

        for (_entity, (global, body)) in cx.world.query_mut::<(&Global3, &RigidBodyHandle)>() {
            let body = data.bodies.get_mut(*body).unwrap();

            if relative_ne!(*body.position(), global.iso) {
                body.set_position(global.iso, true);
            }
        }

        struct SenderEventHandler {
            tx: Sender<ContactEvent>,
            intersection_tx: Sender<IntersectionEvent>,
        }

        impl EventHandler for SenderEventHandler {
            fn handle_intersection_event(&self, _event: IntersectionEvent) {
                self.intersection_tx.send(_event).unwrap();
            }
            fn handle_contact_event(&self, event: ContactEvent, _pair: &ContactPair) {
                self.tx.send(event).unwrap();
            }
        }

        let (tx, rx) = unbounded();
        let (intersection_tx, intersection_rx) = unbounded();

        self.pipeline.step(
            &data.gravity,
            &self.integration_parameters,
            &mut data.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut data.bodies,
            &mut data.colliders,
            &mut data.joints,
            &mut self.ccd_solver,
            &(),
            &SenderEventHandler {
                tx,
                intersection_tx,
            },
        );

        for (_, (global, body)) in cx.world.query_mut::<(&mut Global3, &RigidBodyHandle)>() {
            let body = data.bodies.get_mut(*body).unwrap();
            global.iso = *body.position();
        }

        while let Ok(event) = rx.recv() {
            match event {
                ContactEvent::Started(lhs, rhs) => {
                    let lhs_data =
                        ColliderUserData3::get(data.colliders.get(lhs).unwrap()).unwrap();
                    let rhs_data =
                        ColliderUserData3::get(data.colliders.get(rhs).unwrap()).unwrap();

                    if let Ok(queue) = cx
                        .world
                        .query_one_mut::<&mut ContactQueue3>(&lhs_data.entity)
                    {
                        queue.contacts_started.push(rhs);
                    }

                    if let Ok(queue) = cx
                        .world
                        .query_one_mut::<&mut ContactQueue3>(&rhs_data.entity)
                    {
                        queue.contacts_started.push(lhs);
                    }
                }
                ContactEvent::Stopped(lhs, rhs) => {
                    let lhs_data =
                        ColliderUserData3::get(data.colliders.get(lhs).unwrap()).unwrap();
                    let rhs_data =
                        ColliderUserData3::get(data.colliders.get(rhs).unwrap()).unwrap();

                    if let Ok(queue) = cx
                        .world
                        .query_one_mut::<&mut ContactQueue3>(&lhs_data.entity)
                    {
                        queue.contacts_stopped.push(rhs);
                    }

                    if let Ok(queue) = cx
                        .world
                        .query_one_mut::<&mut ContactQueue3>(&rhs_data.entity)
                    {
                        queue.contacts_stopped.push(lhs);
                    }
                }
            }
        }

        while let Ok(event) = intersection_rx.recv() {
            let lhs = event.collider1;
            let rhs = event.collider2;

            let lhs_data = ColliderUserData3::get(data.colliders.get(lhs).unwrap()).unwrap();
            let rhs_data = ColliderUserData3::get(data.colliders.get(rhs).unwrap()).unwrap();

            if event.intersecting {
                if let Ok(queue) = cx
                    .world
                    .query_one_mut::<&mut IntersectionQueue3>(&lhs_data.entity)
                {
                    queue.intersecting_started.push(rhs);
                }

                if let Ok(queue) = cx
                    .world
                    .query_one_mut::<&mut IntersectionQueue3>(&rhs_data.entity)
                {
                    queue.intersecting_started.push(lhs);
                }
            } else {
                if let Ok(queue) = cx
                    .world
                    .query_one_mut::<&mut IntersectionQueue3>(&lhs_data.entity)
                {
                    queue.intersecting_stopped.push(rhs);
                }

                if let Ok(queue) = cx
                    .world
                    .query_one_mut::<&mut IntersectionQueue3>(&rhs_data.entity)
                {
                    queue.intersecting_stopped.push(lhs);
                }
            }
        }

        data.query_pipeline
            .update(&data.islands, &data.bodies, &data.colliders);
    }
}

pub struct BodyUserData3 {
    pub entity: EntityId,
}

impl BodyUserData3 {
    fn get(body: &RigidBody) -> Option<Self> {
        Self::from_user_data(body.user_data)
    }

    fn set_to(&self, body: &mut RigidBody) {
        body.user_data = self.to_user_data();
    }

    fn to_user_data(&self) -> u128 {
        self.entity.bits() as u128
    }

    fn from_user_data(user_data: u128) -> Option<Self> {
        Some(BodyUserData3 {
            entity: EntityId::from_bits(user_data as u64)?,
        })
    }
}

pub struct ColliderUserData3 {
    pub entity: EntityId,
    pub body_index: usize,
}

impl ColliderUserData3 {
    fn get(collider: &Collider) -> Option<Self> {
        Self::from_user_data(collider.user_data)
    }

    fn set_to(&self, collider: &mut Collider) {
        collider.user_data = self.to_user_data();
    }

    fn to_user_data(&self) -> u128 {
        ((self.body_index as u128) << 64) | (self.entity.bits() as u128)
    }

    fn from_user_data(user_data: u128) -> Option<Self> {
        Some(ColliderUserData3 {
            body_index: (user_data >> 64) as usize,
            entity: EntityId::from_bits(user_data as u64)?,
        })
    }
}
