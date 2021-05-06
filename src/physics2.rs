use {
    crate::{
        scene::Global2,
        system::{System, SystemContext},
    },
    flume::{unbounded, Sender},
    hecs::Entity,
    rapier2d::{
        dynamics::{CCDSolver, IntegrationParameters, JointSet, RigidBodyHandle, RigidBodySet},
        geometry::{
            BroadPhase, ColliderHandle, ColliderSet, ContactEvent, IntersectionEvent, NarrowPhase,
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
    pub joints: JointSet,
    pub gravity: na::Vector2<f32>,
}

impl PhysicsData2 {
    pub fn new() -> Self {
        PhysicsData2 {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
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
        Physics2 {
            pipeline: PhysicsPipeline::new(),
            integration_parameters: IntegrationParameters::default(),
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

        for (entity, (global, body)) in cx.world.query_mut::<(&Global2, &RigidBodyHandle)>() {
            let body = data.bodies.get_mut(*body).unwrap();
            if *body.position() != global.iso {
                body.set_position(global.iso, true);
            }
            if body.user_data == 0 {
                body.user_data = entity.to_bits().into();

                for (index, &collider) in body.colliders().iter().enumerate() {
                    data.colliders.get_mut(collider).unwrap().user_data =
                        ((index as u128) << 64) | entity.to_bits() as u128;
                }
            }
        }

        struct SenderEventHandler {
            tx: Sender<ContactEvent>,
        }

        impl EventHandler for SenderEventHandler {
            fn handle_intersection_event(&self, _event: IntersectionEvent) {}
            fn handle_contact_event(&self, event: ContactEvent) {
                self.tx.send(event).unwrap();
            }
        }

        let (tx, rx) = unbounded();

        self.pipeline.step(
            &data.gravity,
            &self.integration_parameters,
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
