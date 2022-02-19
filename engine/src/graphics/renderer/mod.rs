use edict::{entity::EntityId, world::World};
use scoped_arena::Scope;
use sierra::{Encoder, Extent2d, RenderPassEncoder};

#[cfg(feature = "3d")]
pub mod basic;

#[cfg(feature = "2d")]
pub mod sprite;

#[cfg(feature = "egui")]
pub mod egui;

pub mod simple;

use crate::{assets::Assets, clocks::ClockIndex, resources::Res, viewport::Viewport};

use super::Graphics;

pub struct RendererContext<'a, 'b> {
    /// World on which systems are run.
    pub world: &'a mut World,

    /// Resources map.
    /// All singleton values are stored here and accessible by type.
    pub res: &'a mut Res,

    /// Asset loader.
    /// Assets are loaded asynchronously,
    /// result can be awaited in task. See `spawner` field.
    pub assets: &'a mut Assets,

    /// Arena allocator for allocations in hot-path.
    pub scope: &'b Scope<'b>,

    /// Clock index.
    pub clock: ClockIndex,

    /// Graphics context.
    pub graphics: &'a mut Graphics,
}

impl<'a> RendererContext<'_, 'a> {
    pub fn reborrow(&mut self) -> RendererContext<'_, 'a> {
        RendererContext {
            world: &mut *self.world,
            res: &mut *self.res,
            assets: &mut *self.assets,
            scope: self.scope,
            clock: self.clock,
            graphics: &mut *self.graphics,
        }
    }
}

/// Abstract rendering system.
pub trait Renderer: 'static {
    /// Render into specified viewports.
    fn render(
        &mut self,
        cx: RendererContext<'_, '_>,
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
        cx: RendererContext<'a, 'a>,
        fence_index: usize,
        inputs: <Self as RenderNodeInputs<'a>>::Inputs,
    ) -> eyre::Result<Self::Outputs>;
}

pub trait DrawNode: 'static {
    /// Draw.
    fn draw<'a, 'b: 'a>(
        &'b mut self,
        cx: RendererContext<'a, 'b>,
        fence_index: usize,
        encoder: &mut Encoder<'a>,
        render_pass: &mut RenderPassEncoder<'_, 'b>,
        camera: EntityId,
        viewport: Extent2d,
    ) -> eyre::Result<()>;
}

impl<N> DrawNode for Box<N>
where
    N: DrawNode + ?Sized,
{
    fn draw<'a, 'b: 'a>(
        &'b mut self,
        cx: RendererContext<'a, 'b>,
        fence_index: usize,
        encoder: &mut Encoder<'a>,
        render_pass: &mut RenderPassEncoder<'_, 'b>,
        camera: EntityId,
        viewport: Extent2d,
    ) -> eyre::Result<()> {
        (&mut **self).draw(cx, fence_index, encoder, render_pass, camera, viewport)
    }
}

/// Inputs for render node that simply draws objects.
pub struct DrawNodeInputs<'a> {
    pub encoder: &'a mut Encoder<'a>,
    pub render_pass: RenderPassEncoder<'a, 'a>,
    pub camera: EntityId,
    pub viewport: Extent2d,
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
        cx: RendererContext<'a, 'a>,
        fence_index: usize,
        mut inputs: DrawNodeInputs<'a>,
    ) -> eyre::Result<()> {
        self.draw(
            cx,
            fence_index,
            inputs.encoder,
            &mut inputs.render_pass,
            inputs.camera,
            inputs.viewport,
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
