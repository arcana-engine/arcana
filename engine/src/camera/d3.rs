use crate::{
    command::CommandQueue,
    system::{System, SystemContext},
};

#[cfg(feature = "visible")]
use crate::{
    control::{EventTranslator, InputEvent},
    event::{ElementState, KeyboardInput, VirtualKeyCode},
};

use crate::scene::Global3;

/// Camera in 3 dimensions.
#[derive(Clone, Debug)]
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

#[derive(Clone, Copy, Debug)]
enum Kind {
    Perspective,
    Orthographic,
}

impl Default for Camera3 {
    #[inline]
    fn default() -> Self {
        Camera3::perspective(1.0, std::f32::consts::FRAC_PI_2, 1.0, 100.0)
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

    #[inline]
    pub fn proj(&self) -> &na::Projective3<f32> {
        &self.proj
    }

    /// Update aspect ration of the camera.
    #[inline]
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
        self.update_proj();
    }

    /// Update aspect ration of the camera.
    #[inline]
    pub fn set_fovy(&mut self, fovy: f32) {
        self.fovy = fovy;
        self.update_proj();
    }

    /// Update aspect ration of the camera.
    #[inline]
    pub fn set_znear(&mut self, znear: f32) {
        self.znear = znear;
        self.update_proj();
    }

    /// Update aspect ration of the camera.
    #[inline]
    pub fn set_zfar(&mut self, zfar: f32) {
        self.zfar = zfar;
        self.update_proj();
    }

    /// Converts point in world space into point in screen space.
    /// Screen space Z is depth.
    #[inline]
    pub fn world_to_screen(
        &self,
        view: &na::Affine3<f32>,
        point: &na::Point3<f32>,
    ) -> na::Point3<f32> {
        self.proj
            .transform_point(&view.inverse_transform_point(point))
    }

    /// Converts point in screen space into point in world space.
    /// Screen space Z is depth.
    #[inline]
    pub fn screen_to_world(
        &self,
        view: &na::Affine3<f32>,
        point: &na::Point3<f32>,
    ) -> na::Point3<f32> {
        view.transform_point(&self.proj.inverse_transform_point(point))
    }

    /// Converts point in screen space into ray in world space.
    /// Screen space Z is depth.
    #[cfg(feature = "parry3d")]
    pub fn screen_to_world_ray(
        &self,
        view: &na::Affine3<f32>,
        point: &na::Point2<f32>,
    ) -> parry3d::query::Ray {
        let origin = self.screen_to_world(view, &na::Point3::new(point.x, point.y, 0.0));
        let target = self.screen_to_world(view, &na::Point3::new(point.x, point.y, 1.0));
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

#[cfg(feature = "visible")]
pub struct FreeCamera3Controller {
    pitch: f32,
    yaw: f32,

    forward_pressed: bool,
    backward_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    up_pressed: bool,
    down_pressed: bool,
}

#[cfg(feature = "visible")]
impl Default for FreeCamera3Controller {
    #[inline]
    fn default() -> Self {
        FreeCamera3Controller::new()
    }
}

#[cfg(feature = "visible")]
impl FreeCamera3Controller {
    #[inline]
    pub fn new() -> Self {
        FreeCamera3Controller {
            pitch: 0.0,
            yaw: 0.0,

            forward_pressed: false,
            backward_pressed: false,
            left_pressed: false,
            right_pressed: false,
            up_pressed: false,
            down_pressed: false,
        }
    }
}

#[cfg(feature = "visible")]
impl EventTranslator for FreeCamera3Controller {
    type Command = FreeCamera3Command;

    fn translate(&mut self, event: InputEvent) -> Option<FreeCamera3Command> {
        match event {
            InputEvent::MouseMotion { delta: (x, y) } => {
                self.pitch -= (x * 0.001) as f32;
                self.yaw -= (y * 0.001) as f32;

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
            InputEvent::KeyboardInput(KeyboardInput {
                virtual_keycode: Some(key),
                state,
                ..
            }) => {
                let pressed = matches!(state, ElementState::Pressed);

                match key {
                    VirtualKeyCode::W => self.forward_pressed = pressed,
                    VirtualKeyCode::S => self.backward_pressed = pressed,
                    VirtualKeyCode::A => self.left_pressed = pressed,
                    VirtualKeyCode::D => self.right_pressed = pressed,
                    VirtualKeyCode::LControl => self.up_pressed = pressed,
                    VirtualKeyCode::Space => self.down_pressed = pressed,
                    _ => return None,
                }

                let forward = (self.forward_pressed as u8 as f32) * -na::Vector3::z();
                let backward = (self.backward_pressed as u8 as f32) * na::Vector3::z();
                let left = (self.left_pressed as u8 as f32) * -na::Vector3::x();
                let right = (self.right_pressed as u8 as f32) * na::Vector3::x();
                let up = (self.up_pressed as u8 as f32) * -na::Vector3::y();
                let down = (self.down_pressed as u8 as f32) * na::Vector3::y();

                Some(FreeCamera3Command::Move(
                    forward + backward + left + right + up + down,
                ))
            }
            _ => None,
        }
    }
}

pub struct FreeCamera3 {
    speed: f32,
    mov: na::Vector3<f32>,
}

impl FreeCamera3 {
    pub fn new(speed: f32) -> Self {
        FreeCamera3 {
            speed,
            mov: na::Vector3::zeros(),
        }
    }
}

pub struct FreeCamera3System;

impl System for FreeCamera3System {
    fn name(&self) -> &str {
        "FreeCamera3System"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        let query = cx.world.query_mut::<(
            &mut Global3,
            &mut FreeCamera3,
            &mut CommandQueue<FreeCamera3Command>,
        )>();
        for (_, (global, camera, commands)) in query {
            for cmd in commands.drain() {
                match cmd {
                    FreeCamera3Command::RotateTo(rot) => {
                        global.iso.rotation = rot;
                    }
                    FreeCamera3Command::Move(mov) => {
                        camera.mov = mov * camera.speed;
                    }
                }
            }
            global.iso.translation.vector +=
                global.iso.rotation * camera.mov * cx.clock.delta.as_secs_f32();
        }
    }
}
