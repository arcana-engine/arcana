//! Provides types and functions to deal with various types of cameras.

use crate::{
    control::{CommandQueue, InputCommander},
    event::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode},
    scene::Global3,
    system::{System, SystemContext},
};

/// Camera in 3 dimensions.
#[derive(Debug)]
pub struct Camera3 {
    kind: Kind,

    /// Viewport aspect ratio.
    aspect: f32,

    /// Vertical Field of View
    fovy: f32,

    /// Nearest visible distance
    znear: f32,

    /// Farthest visible distance
    zfar: f32,

    proj: na::Projective3<f32>,
}

#[derive(Debug)]
enum Kind {
    Perspective,
    Orthographic,
}

impl Default for Camera3 {
    fn default() -> Self {
        Camera3::perspective(1.0, std::f32::consts::FRAC_PI_4, 0.1, 1000.0)
    }
}

impl Camera3 {
    /// Constructs perspective [`Camera3`].
    pub fn perspective(aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        let proj = na::Perspective3::new(aspect, fovy, znear, zfar).to_projective();
        Camera3 {
            aspect,
            fovy,
            znear,
            zfar,
            kind: Kind::Perspective,
            proj,
        }
    }

    /// Constructs orthographic [`Camera3`].
    pub fn orthographic(aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        let top = fovy * 0.5;
        let bottom = -top;
        let right = top * aspect * 0.5;
        let left = -right;
        let proj = na::Orthographic3::new(left, right, bottom, top, znear, zfar).to_projective();
        Camera3 {
            aspect,
            fovy,
            znear,
            zfar,
            kind: Kind::Orthographic,
            proj,
        }
    }

    pub fn proj(&self) -> &na::Projective3<f32> {
        &self.proj
    }

    /// Update aspect ration of the camera.
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
        self.update_proj();
    }

    /// Update aspect ration of the camera.
    pub fn set_fovy(&mut self, fovy: f32) {
        self.fovy = fovy;
        self.update_proj();
    }

    /// Update aspect ration of the camera.
    pub fn set_znear(&mut self, znear: f32) {
        self.znear = znear;
        self.update_proj();
    }

    /// Update aspect ration of the camera.
    pub fn set_zfar(&mut self, zfar: f32) {
        self.zfar = zfar;
        self.update_proj();
    }

    /// Converts point in world space into point in screen space.
    /// Screen space Z is depth.
    pub fn world_to_screen(
        &self,
        view: &na::Affine3<f32>,
        point: &na::Point3<f32>,
    ) -> na::Point3<f32> {
        self.proj.transform_point(&view.transform_point(point))
    }

    /// Converts point in screen space into point in world space.
    /// Screen space Z is depth.
    pub fn screen_to_world(
        &self,
        view: &na::Affine3<f32>,
        point: &na::Point3<f32>,
    ) -> na::Point3<f32> {
        view.inverse_transform_point(&self.proj.inverse_transform_point(point))
    }

    /// Converts point in screen space into ray in world space.
    /// Screen space Z is depth.
    pub fn screen_to_world_ray(
        &self,
        view: &na::Affine3<f32>,
        point: &na::Point2<f32>,
    ) -> parry3d::query::Ray {
        let origin = self.screen_to_world(view, &na::Point3::new(point.x, point.x, 0.0));
        let target = self.screen_to_world(view, &na::Point3::new(point.x, point.x, 1.0));
        let dir = (target - origin).normalize();

        parry3d::query::Ray { origin, dir }
    }

    fn update_proj(&mut self) {
        match self.kind {
            Kind::Perspective => {
                self.proj = na::Perspective3::new(self.aspect, self.fovy, self.znear, self.zfar)
                    .to_projective();
            }
            Kind::Orthographic => {
                let top = self.fovy * 0.5;
                let bottom = -top;
                let right = top * self.aspect * 0.5;
                let left = -right;
                self.proj = na::Orthographic3::new(left, right, bottom, top, self.znear, self.zfar)
                    .to_projective();
            }
        }
    }
}

#[derive(Debug)]
pub enum FreeCamera3Command {
    RotateTo(na::UnitQuaternion<f32>),
    Move(na::Vector3<f32>),
}

pub struct FreeCamera3Controller {
    pitch: f32,
    yaw: f32,
}

impl FreeCamera3Controller {
    pub fn new() -> Self {
        FreeCamera3Controller {
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

impl InputCommander for FreeCamera3Controller {
    type Command = FreeCamera3Command;

    fn translate(&mut self, event: DeviceEvent) -> Option<FreeCamera3Command> {
        match event {
            DeviceEvent::MouseMotion { delta: (x, y) } => {
                self.pitch -= (x * 0.01) as f32;
                self.yaw -= (y * 0.01) as f32;

                self.yaw = self.yaw.clamp(
                    std::f32::consts::FRAC_PI_2 * (f32::EPSILON - 1.0),
                    std::f32::consts::FRAC_PI_2 * (1.0 - f32::EPSILON),
                );

                while self.pitch < -std::f32::consts::PI {
                    self.pitch += std::f32::consts::TAU
                }

                while self.pitch > std::f32::consts::PI {
                    self.pitch -= std::f32::consts::TAU
                }

                Some(FreeCamera3Command::RotateTo(
                    na::UnitQuaternion::from_euler_angles(0.0, self.pitch, 0.0)
                        * na::UnitQuaternion::from_euler_angles(self.yaw, 0.0, 0.0),
                ))
            }
            DeviceEvent::Key(KeyboardInput {
                virtual_keycode: Some(key),
                state,
                ..
            }) => {
                let s = match state {
                    ElementState::Pressed => 1.0,
                    ElementState::Released => -1.0,
                };

                let mov = match key {
                    VirtualKeyCode::W => -na::Vector3::z() * s,
                    VirtualKeyCode::S => na::Vector3::z() * s,
                    VirtualKeyCode::A => -na::Vector3::x() * s,
                    VirtualKeyCode::D => na::Vector3::x() * s,
                    VirtualKeyCode::Space => na::Vector3::y() * s,
                    VirtualKeyCode::LControl => -na::Vector3::y() * s,
                    _ => return None,
                };

                Some(FreeCamera3Command::Move(mov))
            }
            _ => None,
        }
    }
}

pub struct FreeCameraSystem;

impl System for FreeCameraSystem {
    fn name(&self) -> &str {
        "FreeCameraSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        const MAX_ROTATION_SPEED: f32 = 0.1;

        let query = cx
            .world
            .query_mut::<(&mut Global3, &mut CommandQueue<FreeCamera3Command>)>();
        for (_, (global, commands)) in query {
            for cmd in commands.drain() {
                match cmd {
                    FreeCamera3Command::RotateTo(rot) => {
                        global.iso.rotation = rot;
                    }
                    FreeCamera3Command::Move(mov) => {
                        global.iso.translation.vector +=
                            global.iso.rotation * mov * cx.clock.delta.as_secs_f32();
                    }
                }
            }

            // global.iso.translation.vector +=
            //     free_camera.rot * free_camera.speed * cx.clock.delta.as_secs_f32() * 5.0;

            // let rot = global.iso.rotation.rotation_to(&free_camera.rot);

            // let max_rotation = MAX_ROTATION_SPEED * cx.clock.delta.as_secs_f32();
            // debug_assert!(max_rotation >= 0.0);

            // let angle = rot.angle();
            // if angle > max_rotation {
            //     let a = (angle - max_rotation).powi(2) * angle.signum()
            //         / ((angle - max_rotation).abs() + 0.1);

            //     let rot =
            //         na::UnitQuaternion::from_axis_angle(&rot.axis().unwrap(), a + max_rotation);
            //     global.iso.append_rotation_wrt_center_mut(&rot);
            // } else {
            //     global.iso.rotation = free_camera.rot;
            // }
        }
        Ok(())
    }
}

/// Camera in 3 dimensions.
#[derive(Debug)]
pub struct Camera2 {
    /// Viewport aspect ratio.
    aspect: f32,

    /// Vertical scale
    scaley: f32,

    affine: na::Affine2<f32>,
}

impl Default for Camera2 {
    fn default() -> Self {
        Camera2::new(1.0, 1.0)
    }
}

impl Camera2 {
    pub fn new(aspect: f32, scaley: f32) -> Self {
        let affine = na::Affine2::from_matrix_unchecked(na::Matrix3::new(
            aspect * scaley,
            0.0,
            0.0,
            0.0,
            scaley,
            0.0,
            0.0,
            0.0,
            1.0,
        ));

        Camera2 {
            aspect,
            scaley,
            affine,
        }
    }

    pub fn affine(&self) -> &na::Affine2<f32> {
        &self.affine
    }

    /// Update aspect ration of the camera.
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
        self.update_affine();
    }

    /// Update aspect ration of the camera.
    pub fn set_scaley(&mut self, scaley: f32) {
        self.scaley = scaley;
        self.update_affine();
    }

    pub fn update_affine(&mut self) {
        self.affine = na::Affine2::from_matrix_unchecked(na::Matrix3::new(
            self.scaley / self.aspect,
            0.0,
            0.0,
            0.0,
            self.scaley,
            0.0,
            0.0,
            0.0,
            1.0,
        ));
    }
}
