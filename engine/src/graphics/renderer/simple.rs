//! Simple renderer made of single node that draws using provided render pass encoder.

use sierra::{ClearColor, ClearDepth, Fence, Format, Image, Layout, Pass, PipelineStages};

use crate::viewport::Viewport;

use super::{DrawNode, Renderer, RendererContext};

#[derive(Pass)]
#[sierra(subpass(color = color, depth = depth))]
struct SimpleRenderPass {
    #[sierra(attachment(store = const Layout::Present, clear = const ClearColor(0.02, 0.03, 0.03, 1.0)))]
    color: Image,

    #[sierra(attachment(clear = const ClearDepth(1.0)))]
    depth: Format,
}

pub struct SimpleRenderer<N> {
    nodes: Vec<N>,
    render_pass: SimpleRenderPassInstance,
    fences: [Option<Fence>; 3],
    fence_index: usize,
}

impl<N> SimpleRenderer<N> {
    pub fn new(node: N) -> Self {
        SimpleRenderer::with_multiple(vec![node])
    }

    pub fn with_multiple(nodes: Vec<N>) -> Self {
        SimpleRenderer {
            nodes,
            render_pass: SimpleRenderPass::instance(),
            fences: [None, None, None],
            fence_index: 0,
        }
    }
}

impl<N> Renderer for SimpleRenderer<N>
where
    N: DrawNode,
{
    fn render(
        &mut self,
        mut cx: RendererContext<'_, '_>,
        viewports: &mut [&mut Viewport],
    ) -> eyre::Result<()> {
        for viewport in viewports {
            let viewport = &mut **viewport;
            if viewport.needs_redraw() {
                self.render(cx.reborrow(), viewport)?;
            }
        }

        Ok(())
    }
}

impl<N> SimpleRenderer<N>
where
    N: DrawNode,
{
    fn render(
        &mut self,
        mut cx: RendererContext<'_, '_>,
        viewport: &mut Viewport,
    ) -> eyre::Result<()> {
        if let Some(fence) = &mut self.fences[self.fence_index] {
            cx.graphics.wait_fences(&mut [fence], true);
            cx.graphics.reset_fences(&mut [fence]);
        }

        let camera = viewport.camera;

        let mut swapchain_image = viewport.acquire_image()?;

        let viewport_extent = swapchain_image.image().info().extent.into_2d();

        let mut render_pass_encoder = cx.graphics.create_encoder(&*cx.scope)?;

        let mut render_pass = render_pass_encoder.with_render_pass(
            &mut self.render_pass,
            &SimpleRenderPass {
                color: swapchain_image.image().clone(),
                depth: Format::D16Unorm,
            },
            cx.graphics,
        )?;

        let mut cbufs = Vec::new_in(cx.scope);

        for node in &mut self.nodes {
            let mut encoder = cx.graphics.create_encoder(&*cx.scope)?;
            node.draw(
                cx.reborrow(),
                &mut encoder,
                &mut render_pass,
                camera,
                viewport_extent,
            )?;
            cbufs.push(encoder.finish());
        }

        drop(render_pass);

        cbufs.push(render_pass_encoder.finish());

        let fence = match &mut self.fences[self.fence_index] {
            Some(fence) => fence,
            None => self.fences[self.fence_index].get_or_insert(cx.graphics.create_fence()?),
        };

        let [wait, signal] = swapchain_image.wait_signal();

        cx.graphics.submit(
            &mut [(PipelineStages::BOTTOM_OF_PIPE, wait)],
            cbufs,
            &mut [signal],
            Some(fence),
            &*cx.scope,
        )?;

        cx.graphics.present(swapchain_image)?;

        Ok(())
    }
}
