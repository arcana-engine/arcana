//! Simple renderer made of single node that draws using provided render pass encoder.

use sierra::{pass, ClearColor, ClearDepth, Fence, Format, Image, Layout, PipelineStageFlags};

use crate::{Renderer, Viewport};

use super::{DrawNode, RendererContext};

#[pass]
#[subpass(color = color, depth = depth)]
struct ForwardRenderPass {
    #[attachment(store(const Layout::Present), clear(const ClearColor(0.2, 0.1, 0.1, 1.0)))]
    color: Image,

    #[attachment(clear(const ClearDepth(1.0)))]
    depth: Format,
}

pub struct ForwardRenderer<N> {
    node: N,
    render_pass: ForwardRenderPassInstance,
    fences: [Option<Fence>; 3],
    fence_index: usize,
}

impl<N> ForwardRenderer<N> {
    pub fn new(node: N) -> Self {
        ForwardRenderer {
            node,
            render_pass: ForwardRenderPass::instance(),
            fences: [None, None, None],
            fence_index: 0,
        }
    }
}

impl<N> Renderer for ForwardRenderer<N>
where
    N: DrawNode,
{
    fn render(
        &mut self,
        mut cx: RendererContext<'_>,
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

impl<N> ForwardRenderer<N>
where
    N: DrawNode,
{
    fn render(&mut self, mut cx: RendererContext<'_>, viewport: &mut Viewport) -> eyre::Result<()> {
        if let Some(fence) = &mut self.fences[self.fence_index] {
            cx.graphics.wait_fences(&mut [fence], true);
            cx.graphics.reset_fences(&mut [fence]);
        }

        let camera = viewport.camera();

        let mut swapchain_image = viewport.acquire_image(true)?;

        let mut encoder = cx.graphics.create_encoder(&*cx.scope)?;
        let mut render_pass_encoder = cx.graphics.create_encoder(&*cx.scope)?;

        let render_pass = render_pass_encoder.with_render_pass(
            &mut self.render_pass,
            &ForwardRenderPass {
                color: swapchain_image.image().clone(),
                depth: Format::D16Unorm,
            },
            cx.graphics,
        )?;

        self.node.draw(
            cx.reborrow(),
            self.fence_index,
            &mut encoder,
            render_pass,
            camera,
        )?;

        let cbufs = [encoder.finish(), render_pass_encoder.finish()];

        let fence = match &mut self.fences[self.fence_index] {
            Some(fence) => fence,
            None => self.fences[self.fence_index].get_or_insert(cx.graphics.create_fence()?),
        };

        let [wait, signal] = swapchain_image.wait_signal();

        cx.graphics.submit(
            &mut [(PipelineStageFlags::BOTTOM_OF_PIPE, wait)],
            std::array::IntoIter::new(cbufs),
            &mut [signal],
            Some(fence),
            &*cx.scope,
        );

        cx.graphics.present(swapchain_image)?;

        Ok(())
    }
}
