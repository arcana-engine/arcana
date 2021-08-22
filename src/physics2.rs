use {
    crate::{
        clocks::TimeSpan,
        scene::Global2,
        system::{System, SystemContext, DEFAULT_TICK_SPAN},
    },
    approx::relative_ne,
    flume::{unbounded, Sender},
    hecs::Entity,
    rapier2d::{
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
    pub joints: JointSet,
    pub gravity: na::Vector2<f32>,
}

impl PhysicsData2 {
    pub fn new() -> Self {
        PhysicsData2 {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            islands: IslandManager::new(),
            joints: JointSet::new(),
            gravity: na::Vector2::default(),
        }
    }
}

impl Default for PhysicsData2 {
    fn default() -> Self {
        Self::new()
    }
}

impl Physics2 {
    pub fn new() -> Self {
        Physics2::with_tick_span(DEFAULT_TICK_SPAN)
    }

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
    fn name(&self) -> &str {
        "Physics"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let data = cx.res.with(PhysicsData2::new);

        let mut remove_bodies = Vec::with_capacity_in(data.bodies.len(), &*cx.scope);
        let world = &*cx.world;
        data.bodies.iter().for_each(|(handle, body)| {
            let e = Entity::from_bits(body.user_data as u64);
            if !world.contains(e) {
                remove_bodies.push(handle);
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

            if body.user_data == 0 {
                body.user_data = entity.to_bits().into();

                for (index, &collider) in body.colliders().iter().enumerate() {
                    data.colliders.get_mut(collider).unwrap().user_data =
                        ((index as u128) << 64) | entity.to_bits() as u128;
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
            tx: Sender<ContactEvent>,
        }

        impl EventHandler for SenderEventHandler {
            fn handle_intersection_event(&self, _event: IntersectionEvent) {}
            fn handle_contact_event(&self, event: ContactEvent, _pair: &ContactPair) {
                self.tx.send(event).unwrap();
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
            &mut data.joints,
            &mut self.ccd_solver,
            &(),
            &SenderEventHandler { tx },
        );

        for (_, (global, body)) in cx.world.query::<(&mut Global2, &RigidBodyHandle)>().iter() {
            let body = data.bodies.get_mut(*body).unwrap();
            global.iso = *body.position();
        }

        while let Ok(event) = rx.recv() {
            match event {
                ContactEvent::Started(lhs, rhs) => {
                    let bits = data.colliders.get(lhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits);

                    if let Ok(mut queue) = cx.world.get_mut::<ContactQueue2>(entity) {
                        queue.contacts_started.push(rhs);
                    }

                    let bits = data.colliders.get(rhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits);

                    if let Ok(mut queue) = cx.world.get_mut::<ContactQueue2>(entity) {
                        queue.contacts_started.push(lhs);
                    }
                }
                ContactEvent::Stopped(lhs, rhs) => {
                    let bits = data.colliders.get(lhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits);

                    if let Ok(mut queue) = cx.world.get_mut::<ContactQueue2>(entity) {
                        queue.contacts_stopped.push(rhs);
                    }

                    let bits = data.colliders.get(rhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits);

                    if let Ok(mut queue) = cx.world.get_mut::<ContactQueue2>(entity) {
                        queue.contacts_stopped.push(lhs);
                    }
                }
            }
        }

        Ok(())
    }
}
