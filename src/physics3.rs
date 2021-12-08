use {
    crate::{
        clocks::TimeSpan,
        scene::Global3,
        system::{System, SystemContext, DEFAULT_TICK_SPAN},
    },
    approx::relative_ne,
    flume::{unbounded, Sender},
    hecs::Entity,
    rapier3d::{
        dynamics::{
            CCDSolver, IntegrationParameters, IslandManager, JointSet, RigidBodyHandle,
            RigidBodySet,
        },
        geometry::{
            BroadPhase, ColliderHandle, ColliderSet, ContactEvent, ContactPair, IntersectionEvent,
            NarrowPhase,
        },
        na,
        pipeline::{EventHandler, PhysicsPipeline},
    },
};

pub use {parry3d::*, rapier3d::*};

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
    pub gravity: na::Vector3<f32>,
}

impl PhysicsData3 {
    pub fn new() -> Self {
        PhysicsData3 {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            islands: IslandManager::new(),
            joints: JointSet::new(),
            gravity: na::Vector3::default(),
        }
    }
}

impl Default for PhysicsData3 {
    fn default() -> Self {
        Self::new()
    }
}

impl Physics3 {
    pub fn new() -> Self {
        Physics3::with_tick_span(DEFAULT_TICK_SPAN)
    }

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
    fn name(&self) -> &str {
        "Physics"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let data = cx.res.with(PhysicsData3::new);

        let mut remove_bodies = Vec::with_capacity_in(data.bodies.len(), &*cx.scope);
        let world = &mut *cx.world;
        data.bodies.iter().for_each(|(handle, body)| {
            if let Some(e) = Entity::from_bits(body.user_data as u64) {
                match world.query_one_mut::<&RigidBodyHandle>(e) {
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

            match Entity::from_bits(body.user_data as u64) {
                Some(e) if e == entity => {}
                _ => {
                    body.user_data = entity.to_bits().get() as u128;

                    for (index, &collider) in body.colliders().iter().enumerate() {
                        data.colliders.get_mut(collider).unwrap().user_data =
                            ((index as u128) << 64) | body.user_data;
                    }
                }
            }
        }

        for (entity, collider) in cx.world.query_mut::<&ColliderHandle>() {
            let collider = data.colliders.get_mut(*collider).unwrap();

            if collider.user_data == 0 {
                collider.user_data = entity.to_bits().into();
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

        for (_, (global, body)) in cx.world.query::<(&mut Global3, &RigidBodyHandle)>().iter() {
            let body = data.bodies.get_mut(*body).unwrap();
            global.iso = *body.position();
        }

        while let Ok(event) = rx.recv() {
            match event {
                ContactEvent::Started(lhs, rhs) => {
                    let bits = data.colliders.get(lhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits).unwrap();

                    if let Ok(mut queue) = cx.world.get_mut::<ContactQueue3>(entity) {
                        queue.contacts_started.push(rhs);
                    }

                    let bits = data.colliders.get(rhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits).unwrap();

                    if let Ok(mut queue) = cx.world.get_mut::<ContactQueue3>(entity) {
                        queue.contacts_started.push(lhs);
                    }
                }
                ContactEvent::Stopped(lhs, rhs) => {
                    let bits = data.colliders.get(lhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits).unwrap();

                    if let Ok(mut queue) = cx.world.get_mut::<ContactQueue3>(entity) {
                        queue.contacts_stopped.push(rhs);
                    }

                    let bits = data.colliders.get(rhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits).unwrap();

                    if let Ok(mut queue) = cx.world.get_mut::<ContactQueue3>(entity) {
                        queue.contacts_stopped.push(lhs);
                    }
                }
            }
        }

        while let Ok(event) = intersection_rx.recv() {
            let lhs = event.collider1;
            let rhs = event.collider2;

            if event.intersecting {
                let bits = data.colliders.get(lhs).unwrap().user_data as u64;
                let entity = Entity::from_bits(bits).unwrap();

                if let Ok(mut queue) = cx.world.get_mut::<IntersectionQueue3>(entity) {
                    queue.intersecting_started.push(rhs);
                }

                let bits = data.colliders.get(rhs).unwrap().user_data as u64;
                let entity = Entity::from_bits(bits).unwrap();

                if let Ok(mut queue) = cx.world.get_mut::<IntersectionQueue3>(entity) {
                    queue.intersecting_started.push(lhs);
                }
            } else {
                let bits = data.colliders.get(lhs).unwrap().user_data as u64;
                let entity = Entity::from_bits(bits).unwrap();

                if let Ok(mut queue) = cx.world.get_mut::<IntersectionQueue3>(entity) {
                    queue.intersecting_stopped.push(rhs);
                }

                let bits = data.colliders.get(rhs).unwrap().user_data as u64;
                let entity = Entity::from_bits(bits).unwrap();

                if let Ok(mut queue) = cx.world.get_mut::<IntersectionQueue3>(entity) {
                    queue.intersecting_stopped.push(lhs);
                }
            }
        }

        Ok(())
    }
}
