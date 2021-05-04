use {
    crate::{
        scene::Global2,
        system::{System, SystemContext},
    },
    rapier2d::{
        dynamics::{CCDSolver, IntegrationParameters, JointSet, RigidBodyHandle, RigidBodySet},
        geometry::{BroadPhase, ColliderSet, NarrowPhase},
        na,
        pipeline::PhysicsPipeline,
    },
};

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
}

impl PhysicsData2 {
    pub fn new() -> Self {
        PhysicsData2 {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: JointSet::new(),
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
        let sets = cx.res.with(PhysicsData2::new);

        for (_, (global, body)) in cx.world.query::<(&Global2, &RigidBodyHandle)>().iter() {
            let body = sets.bodies.get_mut(*body).unwrap();
            if *body.position() != global.iso {
                body.set_position(global.iso, true);
            }
        }

        self.pipeline.step(
            &na::Vector2::new(0.0, 1.0),
            &self.integration_parameters,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut sets.bodies,
            &mut sets.colliders,
            &mut sets.joints,
            &mut self.ccd_solver,
            &(),
            &(),
        );

        for (_, (global, body)) in cx.world.query::<(&mut Global2, &RigidBodyHandle)>().iter() {
            let body = sets.bodies.get_mut(*body).unwrap();
            global.iso = *body.position();
        }

        Ok(())
    }
}
