use approx::relative_ne;
use arcana::{
    edict::entity::EntityId,
    scene::Global2,
    system::{System, SystemContext, DEFAULT_TICK_SPAN},
    TimeSpan,
};
use flume::{unbounded, Sender};
use rapier2d::{
    dynamics::{
        CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet,
        RigidBodyHandle, RigidBodySet,
    },
    geometry::{BroadPhase, ColliderHandle, ColliderSet, CollisionEvent, ContactPair, NarrowPhase},
    na,
    pipeline::{EventHandler, PhysicsPipeline, QueryPipeline},
    prelude::{Collider, RigidBody},
};

pub use {parry2d::*, rapier2d::*};

pub struct ContactQueue2 {
    contacts_started: Vec<ColliderHandle>,
    contacts_stopped: Vec<ColliderHandle>,
}

impl ContactQueue2 {
    pub const fn new() -> Self {
        ContactQueue2 {
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

pub struct IntersectionQueue2 {
    intersecting_started: Vec<ColliderHandle>,
    intersecting_stopped: Vec<ColliderHandle>,
}

impl IntersectionQueue2 {
    pub const fn new() -> Self {
        IntersectionQueue2 {
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

pub struct Physics2 {
    pipeline: PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    ccd_solver: CCDSolver,
}

pub struct PhysicsData2 {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub islands: IslandManager,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub query_pipeline: QueryPipeline,
    pub gravity: na::Vector2<f32>,
}

impl Default for PhysicsData2 {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsData2 {
    #[inline]
    pub fn new() -> Self {
        PhysicsData2 {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            islands: IslandManager::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            query_pipeline: QueryPipeline::new(),
            gravity: na::Vector2::default(),
        }
    }

    pub fn body_user_data(&self, handle: RigidBodyHandle) -> Option<BodyUserData2> {
        let body = self.bodies.get(handle)?;
        BodyUserData2::get(body)
    }

    pub fn collider_user_data(&self, handle: ColliderHandle) -> Option<ColliderUserData2> {
        let collider = self.colliders.get(handle)?;
        ColliderUserData2::get(collider)
    }
}

impl Default for Physics2 {
    #[inline]
    fn default() -> Self {
        Physics2::new()
    }
}

impl Physics2 {
    #[inline]
    pub fn new() -> Self {
        Physics2::with_tick_span(DEFAULT_TICK_SPAN)
    }

    #[inline]
    pub fn with_tick_span(tick_span: TimeSpan) -> Self {
        Physics2 {
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

impl System for Physics2 {
    #[inline]
    fn name(&self) -> &str {
        "Physics"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        let data = cx.res.with(PhysicsData2::new);

        let mut remove_bodies = Vec::with_capacity_in(64, &*cx.scope);
        let world = &mut *cx.world;
        data.bodies.iter().for_each(|(handle, body)| {
            if let Some(body_data) = BodyUserData2::get(body) {
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
                &mut data.impulse_joints,
                &mut data.multibody_joints,
                true,
            );
        }

        for (entity, body) in cx.world.query_mut::<&RigidBodyHandle>() {
            let body = data.bodies.get_mut(*body).unwrap();

            match BodyUserData2::get(body) {
                Some(body_data) if body_data.entity == entity => {}
                _ => {
                    BodyUserData2 { entity }.set_to(body);

                    for (index, &collider) in body.colliders().iter().enumerate() {
                        let collider = data.colliders.get_mut(collider).unwrap();
                        ColliderUserData2 {
                            entity,
                            body_index: index,
                        }
                        .set_to(collider);
                    }
                }
            }
        }

        for (_entity, (global, body)) in cx.world.query_mut::<(&Global2, &RigidBodyHandle)>() {
            let body = data.bodies.get_mut(*body).unwrap();

            if relative_ne!(*body.position(), global.iso) {
                body.set_position(global.iso, true);
            }
        }

        struct SenderEventHandler {
            tx: Sender<CollisionEvent>,
        }

        impl EventHandler for SenderEventHandler {
            fn handle_collision_event(
                &self,
                _bodies: &RigidBodySet,
                _colliders: &ColliderSet,
                event: CollisionEvent,
                _contact_pair: Option<&ContactPair>,
            ) {
                self.tx.send(event).unwrap();
            }

            fn handle_contact_force_event(
                &self,
                _dt: f32,
                _bodies: &RigidBodySet,
                _colliders: &ColliderSet,
                _contact_pair: &ContactPair,
                _total_force_magnitude: f32,
            ) {
                todo!();
            }
        }

        let (tx, rx) = unbounded();

        self.pipeline.step(
            &data.gravity,
            &self.integration_parameters,
            &mut data.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut data.bodies,
            &mut data.colliders,
            &mut data.impulse_joints,
            &mut data.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &SenderEventHandler { tx },
        );

        for (_, (global, body)) in cx.world.query_mut::<(&mut Global2, &RigidBodyHandle)>() {
            let body = data.bodies.get_mut(*body).unwrap();
            global.iso = *body.position();
        }

        while let Ok(event) = rx.recv() {
            match event {
                CollisionEvent::Started(lhs, rhs, _) => {
                    let lhs_data =
                        ColliderUserData2::get(data.colliders.get(lhs).unwrap()).unwrap();

                    let rhs_data =
                        ColliderUserData2::get(data.colliders.get(rhs).unwrap()).unwrap();

                    if let Ok(queue) = cx
                        .world
                        .query_one_mut::<&mut ContactQueue2>(&lhs_data.entity)
                    {
                        queue.contacts_started.push(rhs);
                    }

                    if let Ok(queue) = cx
                        .world
                        .query_one_mut::<&mut ContactQueue2>(&rhs_data.entity)
                    {
                        queue.contacts_started.push(lhs);
                    }
                }
                CollisionEvent::Stopped(lhs, rhs, _) => {
                    let lhs_data =
                        ColliderUserData2::get(data.colliders.get(lhs).unwrap()).unwrap();

                    let rhs_data =
                        ColliderUserData2::get(data.colliders.get(rhs).unwrap()).unwrap();

                    if let Ok(queue) = cx
                        .world
                        .query_one_mut::<&mut ContactQueue2>(&lhs_data.entity)
                    {
                        queue.contacts_stopped.push(rhs);
                    }

                    if let Ok(queue) = cx
                        .world
                        .query_one_mut::<&mut ContactQueue2>(&rhs_data.entity)
                    {
                        queue.contacts_stopped.push(lhs);
                    }
                }
            }
        }

        data.query_pipeline
            .update(&data.islands, &data.bodies, &data.colliders);
    }
}

pub struct BodyUserData2 {
    pub entity: EntityId,
}

impl BodyUserData2 {
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
        Some(BodyUserData2 {
            entity: EntityId::from_bits(user_data as u64)?,
        })
    }
}

pub struct ColliderUserData2 {
    pub entity: EntityId,
    pub body_index: usize,
}

impl ColliderUserData2 {
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
        Some(ColliderUserData2 {
            body_index: (user_data >> 64) as usize,
            entity: EntityId::from_bits(user_data as u64)?,
        })
    }
}
