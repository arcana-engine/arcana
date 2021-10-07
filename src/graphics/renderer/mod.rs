use hecs::Entity;
use sierra::{Encoder, RenderPassEncoder};

#[cfg(feature = "3d")]
pub mod basic;

#[cfg(feature = "2d")]
pub mod sprite;

pub mod forward;

use {
    super::Graphics,
    crate::{clocks::ClockIndex, resources::Res, viewport::Viewport},
    hecs::World,
    scoped_arena::Scope,
};

pub struct RendererContext<'a> {
    pub world: &'a mut World,
    pub res: &'a mut Res,
    pub graphics: &'a mut Graphics,
    pub scope: &'a Scope<'a>,
    pub clock: ClockIndex,
}

impl RendererContext<'_> {
    pub fn reborrow(&mut self) -> RendererContext<'_> {
        RendererContext {
            world: &mut *self.world,
            res: &mut *self.res,
            graphics: &mut *self.graphics,
            scope: self.scope,
            clock: self.clock,
        }
    }
}

/// Abstract rendering system.
pub trait Renderer: 'static {
    /// Render into specified viewports.
    fn render(
        &mut self,
        cx: RendererContext<'_>,
        viewports: &mut [&mut Viewport],
    ) -> eyre::Result<()>;
}

pub trait RenderNodeInputs<'a> {
    /// Inputs required by render pass.
    /// This includes resources to which render pass will write.
    type Inputs;
}

/// Single render node.
/// Renderer may consist of few of them.
/// Rarely used in generic way.
/// This trait's main purpose is to define API for render nodes to implement.
pub trait RenderNode: for<'a> RenderNodeInputs<'a> + 'static {
    /// Outputs produced by render pass.
    /// This includes any resources render pass creates.
    /// This does not include existing resources to which render pass will write.
    type Outputs;

    /// Render using inputs and producing outputs.
    fn render<'a>(
        &'a mut self,
        cx: RendererContext<'a>,
        fence_index: usize,
        inputs: <Self as RenderNodeInputs<'a>>::Inputs,
    ) -> eyre::Result<Self::Outputs>;
}

/// Inputs for render node that simply draws objects.
pub struct DrawNodeInputs<'a> {
    pub encoder: &'a mut Encoder<'a>,
    pub render_pass: RenderPassEncoder<'a, 'a>,
    pub camera: Entity,
}

pub trait DrawNode: 'static {
    /// Draw.
    fn draw<'a>(
        &'a mut self,
        cx: RendererContext<'a>,
        fence_index: usize,
        encoder: &mut Encoder<'a>,
        render_pass: RenderPassEncoder<'_, 'a>,
        camera: Entity,
    ) -> eyre::Result<()>;
}

impl<'a, N> RenderNodeInputs<'a> for N
where
    N: DrawNode,
{
    type Inputs = DrawNodeInputs<'a>;
}

impl<N> RenderNode for N
where
    N: DrawNode,
{
    type Outputs = ();

    fn render<'a>(
        &'a mut self,
        cx: RendererContext<'a>,
        fence_index: usize,
        inputs: DrawNodeInputs<'a>,
    ) -> eyre::Result<()> {
        self.draw(
            cx,
            fence_index,
            inputs.encoder,
            inputs.render_pass,
            inputs.camera,
        )
    }
}

#[allow(unused)]
#[inline(always)]
fn mat4_na_to_sierra(m: na::Matrix4<f32>) -> sierra::mat4<f32> {
    let array: [[f32; 4]; 4] = m.into();
    sierra::mat4::from(array)
}

#[allow(unused)]
#[inline(always)]
fn mat3_na_to_sierra(m: na::Matrix3<f32>) -> sierra::mat3<f32> {
    let array: [[f32; 3]; 3] = m.into();
    sierra::mat3::from(array)
}
