pub mod basic;
pub mod sprite;

use {
    super::Graphics,
    crate::{clocks::ClockIndex, resources::Res, viewport::Viewport},
    bumpalo::Bump,
    hecs::World,
};

pub struct RendererContext<'a> {
    pub world: &'a mut World,
    pub res: &'a mut Res,
    pub graphics: &'a mut Graphics,
    pub bump: &'a Bump,
    pub clock: ClockIndex,
}

impl RendererContext<'_> {
    fn reborrow(&mut self) -> RendererContext<'_> {
        RendererContext {
            world: &mut *self.world,
            res: &mut *self.res,
            graphics: &mut *self.graphics,
            bump: &*self.bump,
            clock: self.clock,
        }
    }
}

/// Abstract rendering system.
pub trait Renderer: 'static {
    fn new(graphics: &mut Graphics) -> eyre::Result<Self>
    where
        Self: Sized;

    /// Render into specified viewports.
    fn render(
        &mut self,
        cx: RendererContext<'_>,
        viewports: &mut [&mut Viewport],
    ) -> eyre::Result<()>;
}

#[inline(always)]
fn mat4_na_to_sierra(m: na::Matrix4<f32>) -> sierra::mat4<f32> {
    let array: [[f32; 4]; 4] = m.into();
    sierra::mat4::from(array)
}

#[inline(always)]
fn mat3_na_to_sierra(m: na::Matrix3<f32>) -> sierra::mat3<f32> {
    let array: [[f32; 3]; 3] = m.into();
    sierra::mat3::from(array)
}
