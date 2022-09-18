use edict::prelude::Component;

use crate::rect::Rect;

/// Camera in 2 dimensions.
#[derive(Debug, Component)]
pub struct Camera2 {
    /// Vertical scale
    scaley: f32,
}

impl Default for Camera2 {
    #[inline]
    fn default() -> Self {
        Camera2::new(1.0)
    }
}

impl Camera2 {
    pub fn new(scaley: f32) -> Self {
        Camera2 { scaley }
    }

    pub fn affine(&self, aspect: f32) -> na::Affine2<f32> {
        na::Affine2::from_matrix_unchecked(na::Matrix3::new(
            self.scaley / aspect,
            0.0,
            0.0,
            0.0,
            self.scaley,
            0.0,
            0.0,
            0.0,
            1.0,
        ))
    }

    pub fn scale(&self, aspect: f32) -> na::Vector2<f32> {
        na::Vector2::new(self.scaley * aspect, self.scaley)
    }

    pub fn inverse_scale(&self, aspect: f32) -> na::Vector2<f32> {
        na::Vector2::new(1.0 / self.scaley / aspect, 1.0 / self.scaley)
    }

    /// Update aspect ration of the camera.
    pub fn set_scaley(&mut self, scaley: f32) {
        self.scaley = scaley;
    }

    /// Converts point in screen space into point in world space.
    pub fn screen_to_world(
        &self,
        iso: &na::Isometry2<f32>,
        point: &na::Point2<f32>,
        aspect: f32,
    ) -> na::Point2<f32> {
        let scale = self.scale(aspect);
        iso.inverse_transform_point(&na::Point2::from(point.coords.component_mul(&scale)))
    }

    /// Converts point in world space into point in screen space.
    pub fn world_to_screen(
        &self,
        iso: &na::Isometry2<f32>,
        point: &na::Point2<f32>,
        aspect: f32,
    ) -> na::Point2<f32> {
        let inverse_scale = self.inverse_scale(aspect);
        iso.transform_point(&na::Point2::from(
            point.coords.component_mul(&inverse_scale),
        ))
    }

    pub fn transform_aabb(&self, iso: &na::Isometry2<f32>, aabb: &Rect, aspect: f32) -> Rect {
        let scale = self.scale(aspect);

        let top_left = iso.inverse_transform_point(&na::Point2::from(
            aabb.top_left().coords.component_mul(&scale),
        ));
        let bottom_left = iso.inverse_transform_point(&na::Point2::from(
            aabb.bottom_left().coords.component_mul(&scale),
        ));
        let top_right = iso.inverse_transform_point(&na::Point2::from(
            aabb.top_right().coords.component_mul(&scale),
        ));
        let bottom_right = iso.inverse_transform_point(&na::Point2::from(
            aabb.bottom_right().coords.component_mul(&scale),
        ));

        let xs = [top_left.x, bottom_left.x, top_right.x, bottom_right.x];
        let left = xs.into_iter().reduce(f32::min).unwrap();
        let right = xs.into_iter().reduce(f32::max).unwrap();

        let ys = [top_left.y, bottom_left.y, top_right.y, bottom_right.y];
        let top = ys.into_iter().reduce(f32::max).unwrap();
        let bottom = ys.into_iter().reduce(f32::min).unwrap();

        Rect {
            left,
            right,
            top,
            bottom,
        }
    }

    pub fn view_aabb(&self, iso: &na::Isometry2<f32>, aspect: f32) -> Rect {
        self.transform_aabb(
            iso,
            &Rect {
                left: -1.0,
                right: 1.0,
                top: 1.0,
                bottom: -1.0,
            },
            aspect,
        )
    }
}
