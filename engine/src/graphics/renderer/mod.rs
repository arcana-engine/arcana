use std::collections::VecDeque;

use edict::{
    entity::EntityId, epoch::EpochId, query::QueryBorrowAny, world::World, Entities, State,
};
use hashbrown::HashSet;
use scoped_arena::Scope;
use sierra::{CommandBuffer, Encoder, Extent2, Fence, PipelineStages, RenderPassEncoder};

use crate::scoped_allocator::ScopedAllocator;

use super::{Graphics, NeedsRedraw, RenderTarget, RendersTo, SurfaceSwapchain};

#[cfg(feature = "3d")]
pub mod basic;

// #[cfg(feature = "2d")]
// pub mod sprite;

// #[cfg(feature = "with-egui")]
// pub mod egui;

// pub mod simple;

/// Abstract rendering system.
pub trait Renderer: Send + 'static {
    /// Runs rendering.
    ///
    /// Provided closure `dependencies` should be called with id of entities that owns render target
    ///
    /// Returns list of generated command buffers that should be submitted to the graphics queue
    /// after command buffers created by dependencies.
    fn render(
        &mut self,
        world: &World,
        scope: &Scope,
        dependencies: &mut dyn FnMut(EntityId),
    ) -> eyre::Result<Vec<CommandBuffer>>;
}

pub trait RenderNodeInputs<'a> {
    /// Inputs required by render pass.
    /// This includes resources to which render pass will write.
    type Inputs;
}

pub struct RenderContext<'a, 'b> {
    pub world: &'a mut World,
    pub scope: &'a Scope<'b>,
    pub dependencies: &'a mut dyn FnMut(EntityId),
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
        cx: RenderContext<'a, '_>,
        inputs: <Self as RenderNodeInputs<'a>>::Inputs,
    ) -> eyre::Result<Self::Outputs>;
}

/// A render node to record draw calls.
pub trait DrawNode: 'static {
    /// Draw.
    fn draw<'a, 'b: 'a>(
        &'b mut self,
        cx: RenderContext<'a, 'b>,
        encoder: &mut Encoder<'a>,
        render_pass: &mut RenderPassEncoder<'_, 'b>,
        camera: EntityId,
        viewport: Extent2,
    ) -> eyre::Result<()>;
}

impl<N> DrawNode for Box<N>
where
    N: DrawNode + ?Sized,
{
    fn draw<'a, 'b: 'a>(
        &'b mut self,
        cx: RenderContext<'a, 'b>,

        encoder: &mut Encoder<'a>,
        render_pass: &mut RenderPassEncoder<'_, 'b>,
        camera: EntityId,
        viewport: Extent2,
    ) -> eyre::Result<()> {
        (&mut **self).draw(cx, encoder, render_pass, camera, viewport)
    }
}

/// Inputs for [`DrawNode`].
pub struct DrawNodeInputs<'a> {
    pub encoder: &'a mut Encoder<'a>,
    pub render_pass: RenderPassEncoder<'a, 'a>,
    pub camera: EntityId,
    pub viewport: Extent2,
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
        cx: RenderContext<'a, '_>,
        mut inputs: DrawNodeInputs<'a>,
    ) -> eyre::Result<()> {
        self.draw(
            cx,
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

struct RenderSystemState {
    epoch: EpochId,
    fences: Vec<Fence>,
    next_fence: usize,
}

const FENCE_COUNT: usize = 2;

impl RenderSystemState {
    fn fence(&mut self, graphics: &Graphics) -> &mut Fence {
        if self.fences.len() <= self.next_fence {
            self.fences.push(graphics.create_fence().unwrap());
        }

        let fence = &mut self.fences[self.next_fence];

        self.next_fence = (self.next_fence + 1) % FENCE_COUNT;

        if self.fences.len() <= self.next_fence {
            graphics.wait_fences(&mut [&mut self.fences[self.next_fence]], false);
        }

        fence
    }
}

/// System that run renderers.
///
/// Lookups for render targets that should be updated and runs
/// all associated renderers and transitively for all dependencies.
pub fn rendering_system(
    allocator: &mut ScopedAllocator,
    world: &mut World,
    mut state: State<RenderSystemState>,
) {
    let mut graphics = world.expect_resource_mut::<Graphics>();

    let mut swapchain_images = Vec::new_in(&**allocator);
    let mut render_queue = Vec::new_in(&**allocator);

    let mut surfaces = world
        .query_mut::<(Entities, &mut SurfaceSwapchain)>()
        .related::<RendersTo>()
        .modified::<&NeedsRedraw>(state.epoch);

    for ((entity, surface), renderers, NeedsRedraw) in surfaces.iter_mut() {
        let swapchain_image = match surface.swapchain.acquire_image() {
            Err(err) => panic!("{}", err),
            Ok(swapchain_image) => swapchain_image,
        };

        let mut rt = world.query_one::<&mut RenderTarget>(entity).unwrap();
        rt.get()
            .unwrap()
            .set_swapchain_image(swapchain_image.image().clone());

        swapchain_images.push(swapchain_image);
        render_queue.extend_from_slice(renderers);
    }

    render_queue.sort_unstable_by_key(|e| e.id());
    render_queue.dedup();

    enum MaybeExecutedRender {
        Render(EntityId),
        Executed {
            id: EntityId,
            buffers: Vec<CommandBuffer>,
        },
    }

    let mut render_queue =
        VecDeque::from_iter(render_queue.into_iter().map(MaybeExecutedRender::Render));

    let mut executed_renderers = HashSet::new_in(&**allocator);
    let mut pending_renderers = HashSet::new_in(&**allocator);
    let mut command_queue = Vec::new_in(&**allocator);

    while let Some(render) = render_queue.pop_front() {
        match render {
            MaybeExecutedRender::Render(render_id) => {
                if executed_renderers.contains(&render_id) {
                    continue;
                }

                let mut render = world
                    .query_one_mut::<QueryBorrowAny<&mut (dyn Renderer)>>(render_id)
                    .unwrap();

                let mut deps = Vec::new_in(&**allocator);
                let command_buffers = render
                    .render(world, &allocator, &mut |dep| deps.push(dep))
                    .unwrap();

                deps.sort_unstable_by_key(|e| e.id());
                deps.dedup();

                let mut dep_renders = Vec::new_in(&**allocator);

                let renders_to = world.new_query_mut().related::<RendersTo>();
                for dep in deps {
                    if let Some(renders) = renders_to.get_one(dep).ok() {
                        for render in renders {
                            if executed_renderers.contains(render) {
                                continue;
                            }
                            if pending_renderers.contains(render) {
                                panic!("Cyclic dependency");
                            }
                            dep_renders.push(*render);
                        }
                    }
                }

                drop(deps);
                drop(renders_to);

                dep_renders.sort_unstable_by_key(|e| e.id());
                dep_renders.dedup();

                if dep_renders.is_empty() {
                    command_queue.extend(command_buffers);
                    executed_renderers.insert(render_id);
                } else {
                    pending_renderers.insert(render_id);

                    render_queue.push_front(MaybeExecutedRender::Executed {
                        id: render_id,
                        buffers: command_buffers,
                    });

                    for render in dep_renders {
                        render_queue.push_front(MaybeExecutedRender::Render(render));
                    }
                }
            }
            MaybeExecutedRender::Executed { id, buffers } => {
                // All deps are executed
                executed_renderers.insert(id);
                command_queue.extend(buffers);
            }
        }
    }

    let mut waits = Vec::new_in(&**allocator);
    let mut signals = Vec::new_in(&**allocator);

    for swapchain_image in &mut swapchain_images {
        let [wait, signal] = swapchain_image.wait_signal();
        waits.push((PipelineStages::COLOR_ATTACHMENT_OUTPUT, wait));
        signals.push(signal);
    }

    graphics.submit(
        &mut waits,
        command_queue,
        &mut signals,
        Some(state.fence(&graphics)),
        &**allocator,
    );

    for swapchain_image in swapchain_images {
        graphics.queue.present(swapchain_image);
    }
}
