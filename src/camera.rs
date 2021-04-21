//! Provides types and functions to deal with various types of cameras.

use crate::{
    control::{ControlResult, InputController},
    event::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode},
    scene::Global3,
    system::{System, SystemContext},
};

/// Camera in 3 dimensions.
#[derive(Debug)]
pub struct Camera3d {
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

impl Default for Camera3d {
    fn default() -> Self {
        Camera3d::perspective(1.0, std::f32::consts::FRAC_PI_4, 0.1, 1000.0)
    }
}

impl Camera3d {
    /// Constructs perspective [`Camera3d`].
    pub fn perspective(aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        let proj = na::Perspective3::new(aspect, fovy, znear, zfar).to_projective();
        Camera3d {
            aspect,
            fovy,
            znear,
            zfar,
            kind: Kind::Perspective,
            proj,
        }
    }

    /// Constructs orthographic [`Camera3d`].
    pub fn orthographic(aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        let top = fovy * 0.5;
        let bottom = -top;
        let right = top * aspect * 0.5;
        let left = -right;
        let proj = na::Orthographic3::new(left, right, bottom, top, znear, zfar).to_projective();
        Camera3d {
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

#[derive(Debug, Default)]
pub struct FreeCamera {
    rot: na::UnitQuaternion<f32>,
    speed: na::Vector3<f32>,
}

pub struct FreeCameraController {
    pitch: f32,
    yaw: f32,
}

impl FreeCameraController {
    pub fn new() -> Self {
        FreeCameraController {
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

impl InputController for FreeCameraController {
    type Controlled = FreeCamera;

    fn control(&mut self, event: DeviceEvent, free_camera: &mut FreeCamera) -> ControlResult {
        match event {
            DeviceEvent::MouseMotion { delta: (x, y) } => {
                self.pitch -= x as f32 * 0.01;
                self.yaw -= y as f32 * 0.01;

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

                free_camera.rot = na::UnitQuaternion::from_euler_angles(0.0, self.pitch, 0.0)
                    * na::UnitQuaternion::from_euler_angles(self.yaw, 0.0, 0.0);

                ControlResult::Consumed
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

                let dir = match key {
                    VirtualKeyCode::W => -na::Vector3::z() * s,
                    VirtualKeyCode::S => na::Vector3::z() * s,
                    VirtualKeyCode::A => -na::Vector3::x() * s,
                    VirtualKeyCode::D => na::Vector3::x() * s,
                    VirtualKeyCode::Space => na::Vector3::y() * s,
                    VirtualKeyCode::LControl => -na::Vector3::y() * s,
                    _ => return ControlResult::Ignored,
                };

                free_camera.speed += dir;

                ControlResult::Consumed
            }
            _ => ControlResult::Ignored,
        }
    }

    fn controlled(&self) -> Self::Controlled {
        FreeCamera::default()
    }
}

pub struct FreeCameraSystem;

impl System for FreeCameraSystem {
    fn name(&self) -> &str {
        "FreeCameraSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let query = cx.world.query_mut::<(&mut Global3, &mut FreeCamera)>();
        for (_, (global, free_camera)) in query {
            global.iso.translation.vector +=
                free_camera.rot * free_camera.speed * cx.clock.delta.as_secs_f32() * 5.0;
            global.iso.rotation = free_camera.rot;
        }
        Ok(())
    }
}
