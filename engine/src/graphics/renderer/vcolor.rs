use {
    super::{mat4_na_to_sierra, Renderer, RendererContext},
    crate::{
        camera::Camera3,
        graphics::{
            mesh::Mesh,
            vertex::{Color, Normal3, Position3, VertexType as _},
            Graphics, Scale,
        },
        scene::Global3,
        viewport::Viewport,
    },
    sierra::{
        descriptors, graphics_pipeline_desc, mat4, pass, pipeline, shader_repr, ClearColor,
        ClearDepth, DepthTest, DescriptorsInput, DynamicGraphicsPipeline, Fence, Format,
        FragmentShader, Image, Layout, PipelineInput, PipelineStageFlags, ShaderModuleInfo,
        VertexInputAttribute, VertexInputBinding, VertexInputRate, VertexShader,
    },
};

pub struct VcolorRenderer {
    pipeline_layout: <VcolorPipeline as PipelineInput>::Layout,
    pipeline: DynamicGraphicsPipeline,
    render_pass: VcolorRenderPassInstance,
    fences: [Option<Fence>; 3],
    fence_index: usize,
}

#[pass]
#[subpass(color = color, depth = depth)]
struct VcolorRenderPass {
    #[attachment(store(const Layout::Present), clear(const ClearColor(0.2, 0.1, 0.1, 1.0)))]
    color: Image,

    #[attachment(clear(const ClearDepth(1.0)))]
    depth: Format,
}

#[shader_repr]
#[derive(Clone, Copy)]
struct Uniforms {
    camera_view: mat4,
    camera_proj: mat4,
    transform: mat4,
}

impl Default for Uniforms {
    #[inline]
    fn default() -> Self {
        Uniforms {
            camera_view: mat4::default(),
            camera_proj: mat4::default(),
            transform: mat4::default(),
        }
    }
}

#[descriptors]
struct VcolorDescriptors {
    #[uniform]
    #[stages(Vertex, Fragment)]
    uniforms: Uniforms,
}

#[pipeline]
struct VcolorPipeline {
    #[set]
    set: VcolorDescriptors,
}

struct VcolorRenderable {
    descriptors: <VcolorDescriptors as DescriptorsInput>::Instance,
}

impl VcolorRenderer {
    fn render(&mut self, cx: RendererContext<'_>, viewport: &mut Viewport) -> eyre::Result<()> {
        if let Some(fence) = &mut self.fences[self.fence_index] {
            cx.graphics.wait_fences(&mut [fence], true);
            cx.graphics.reset_fences(&mut [fence]);
        }

        let view = cx
            .world
            .get_mut::<Global3>(viewport.camera())?
            .iso
            .inverse()
            .to_homogeneous();

        let proj = cx
            .world
            .get_mut::<Camera3>(viewport.camera())?
            .proj()
            .to_homogeneous();

        let mut swapchain_image = viewport.acquire_image(true)?;

        let mut uniforms = Uniforms::default();
        uniforms.camera_view = mat4_na_to_sierra(view);
        uniforms.camera_proj = mat4_na_to_sierra(proj);

        let mut new_entities = Vec::new_in(&*cx.scope);

        for (e, ()) in cx
            .world
            .query_mut::<()>()
            .with::<Mesh>()
            .with::<Global3>()
            .without::<VcolorRenderable>()
        {
            new_entities.push(e);
        }

        for e in new_entities {
            cx.world
                .insert_one(
                    e,
                    VcolorRenderable {
                        descriptors: self.pipeline_layout.set.instance(),
                    },
                )
                .unwrap();
        }

        let mut encoder = cx.graphics.create_encoder(&*cx.scope)?;
        let mut render_pass_encoder = cx.graphics.create_encoder(&*cx.scope)?;

        let mut render_pass = render_pass_encoder.with_render_pass(
            &mut self.render_pass,
            &VcolorRenderPass {
                color: swapchain_image.image().clone(),
                depth: Format::D16Unorm,
            },
            cx.graphics,
        )?;

        render_pass.bind_dynamic_graphics_pipeline(&mut self.pipeline, cx.graphics)?;

        let mut writes = Vec::new_in(&*cx.scope);
        for (_, (mesh, global, renderable, scale)) in
            cx.world
                .query_mut::<(&Mesh, &Global3, &mut VcolorRenderable, Option<&Scale>)>()
        {
            match scale {
                Some(scale) => {
                    let m = na::Matrix4::<f32>::new_nonuniform_scaling(&scale.0);
                    uniforms.transform = mat4_na_to_sierra(global.iso.to_homogeneous() * m);
                }
                None => {
                    uniforms.transform = mat4_na_to_sierra(global.iso.to_homogeneous());
                }
            }

            let updated = renderable.descriptors.update(
                &VcolorDescriptors { uniforms },
                cx.graphics,
                &mut writes,
                &mut encoder,
            )?;

            render_pass.bind_graphics_descriptors(&self.pipeline_layout, updated);

            let drawn = mesh.draw(
                0..1,
                &[Position3::layout(), Normal3::layout(), Color::layout()],
                &mut render_pass,
                cx.scope,
            );

            if drawn {
                tracing::info!("Mesh drawn");
            } else {
                tracing::warn!("Mesh is not drawn");
            }
        }

        cx.graphics.update_descriptor_sets(&writes, &[]);

        let fence = match &mut self.fences[self.fence_index] {
            Some(fence) => fence,
            None => self.fences[self.fence_index].get_or_insert(cx.graphics.create_fence()?),
        };

        drop(render_pass);

        let [wait, signal] = swapchain_image.wait_signal();

        cx.graphics.submit(
            &mut [(PipelineStageFlags::BOTTOM_OF_PIPE, wait)],
            std::array::IntoIter::new([encoder.finish(), render_pass_encoder.finish()]),
            &mut [signal],
            Some(fence),
            cx.scope,
        );

        cx.graphics.present(swapchain_image)?;

        Ok(())
    }
}

impl Renderer for VcolorRenderer {
    fn new(graphics: &mut Graphics) -> eyre::Result<Self> {
        let shader_module = graphics.create_shader_module(ShaderModuleInfo::wgsl(
            std::include_bytes!("vcolor.wgsl")
                .to_vec()
                .into_boxed_slice(),
        ))?;

        let pipeline_layout = VcolorPipeline::layout(graphics)?;

        Ok(VcolorRenderer {
            pipeline: DynamicGraphicsPipeline::new(graphics_pipeline_desc! {
                vertex_bindings: vec![
                    VertexInputBinding {
                        rate: VertexInputRate::Vertex,
                        stride: 12,
                    },
                    VertexInputBinding {
                        rate: VertexInputRate::Vertex,
                        stride: 12,
                    },
                    VertexInputBinding {
                        rate: VertexInputRate::Vertex,
                        stride: 16,
                    },
                ],
                vertex_attributes: vec![
                    VertexInputAttribute { location: 0, format: Format::RGB32Sfloat, binding: 0, offset: 0 },
                    VertexInputAttribute { location: 1, format: Format::RGB32Sfloat, binding: 1, offset: 0 },
                    VertexInputAttribute { location: 2, format: Format::RGBA32Sfloat, binding: 2, offset: 0 },
                ],
                vertex_shader: VertexShader::new(shader_module.clone(), "vs_main"),
                fragment_shader: Some(FragmentShader::new(shader_module.clone(), "fs_main")),
                layout: pipeline_layout.raw().clone(),
                depth_test: Some(DepthTest::LESS_WRITE),
            }),
            fences: [None, None, None],
            fence_index: 0,
            render_pass: VcolorRenderPass::instance(),
            pipeline_layout,
        })
    }

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
