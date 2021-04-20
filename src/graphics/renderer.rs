use {
    super::{
        material::Material,
        mesh::Mesh,
        vertex::{Normal3d, Position3d, VertexType as _, UV},
        Graphics, Scale,
    },
    crate::{
        camera::Camera3d, clocks::ClockIndex, resources::Res, scene::Global3, viewport::Viewport,
    },
    bumpalo::{collections::Vec as BVec, Bump},
    hecs::World,
    sierra::{
        descriptors, graphics_pipeline_desc, mat4, pass, pipeline, shader_repr, ClearColor,
        ClearDepth, DepthTest, DescriptorsInput, DynamicGraphicsPipeline, Fence, Format,
        FragmentShader, Image, ImageView, Layout, PipelineInput, PipelineStageFlags, Sampler,
        ShaderModuleInfo, VertexInputAttribute, VertexInputBinding, VertexInputRate, VertexShader,
    },
};

pub struct RendererContext<'a> {
    pub res: &'a mut Res,
    pub world: &'a mut World,
    pub graphics: &'a mut Graphics,
    pub bump: &'a Bump,
    pub clock: ClockIndex,
}

impl RendererContext<'_> {
    fn reborrow(&mut self) -> RendererContext<'_> {
        RendererContext {
            res: &mut *self.res,
            world: &mut *self.world,
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

pub struct BasicRenderer {
    pipeline_layout: <BasicPipeline as PipelineInput>::Layout,
    pipeline: DynamicGraphicsPipeline,
    render_pass: BasicRenderPassInstance,
    fences: [Option<Fence>; 3],
    fence_index: usize,
}

#[pass]
#[subpass(color = color, depth = depth)]
struct BasicRenderPass {
    #[attachment(store(const Layout::Present), clear(const ClearColor(0.2, 0.1, 0.1, 1.0)))]
    color: Image,

    #[attachment(clear(const ClearDepth(1.0)))]
    depth: Format,
}

#[shader_repr]
#[derive(Clone, Copy, Default)]
struct Camera {
    view: mat4,
    proj: mat4,
}

#[shader_repr]
#[derive(Clone, Copy)]
struct BasicGlobals {
    camera: Camera,
    transform: mat4,
    joints: [mat4; 128],
}

impl Default for BasicGlobals {
    fn default() -> Self {
        BasicGlobals {
            camera: Default::default(),
            transform: Default::default(),
            joints: [Default::default(); 128],
        }
    }
}

#[descriptors]
struct BasicDescriptors {
    #[sampler]
    #[stages(Fragment)]
    sampler: Sampler,

    #[sampled_image]
    #[stages(Fragment)]
    albedo: ImageView,

    #[uniform]
    #[stages(Vertex)]
    globals: BasicGlobals,
}

#[pipeline]
struct BasicPipeline {
    #[set]
    set: BasicDescriptors,
}

struct BasicRenderable {
    descriptors: <BasicDescriptors as DescriptorsInput>::Instance,
}

impl BasicRenderer {
    fn render(&mut self, cx: RendererContext<'_>, viewport: &mut Viewport) -> eyre::Result<()> {
        if let Some(fence) = &self.fences[self.fence_index] {
            cx.graphics.wait_fences(&[fence], true);
            cx.graphics.reset_fences(&[fence]);
        }

        let swapchain_image = viewport.acquire_image(true)?;

        let view = cx
            .world
            .get_mut::<Global3>(viewport.camera())?
            .iso
            .inverse()
            .to_homogeneous();

        let proj = cx
            .world
            .get_mut::<Camera3d>(viewport.camera())?
            .proj()
            .to_homogeneous();

        let mut globals = BasicGlobals::default();
        globals.camera.view = mat4_na_to_sierra(view);
        globals.camera.proj = mat4_na_to_sierra(proj);

        let mut new_entities = BVec::new_in(cx.bump);

        for (e, ()) in cx
            .world
            .query_mut::<()>()
            .with::<Mesh>()
            .with::<Material>()
            .with::<Global3>()
            .without::<BasicRenderable>()
        {
            new_entities.push(e);
        }

        for e in new_entities {
            cx.world
                .insert_one(
                    e,
                    BasicRenderable {
                        descriptors: self.pipeline_layout.set.instance(),
                    },
                )
                .unwrap();
        }

        let mut encoder = cx.graphics.create_encoder(cx.bump)?;
        let mut render_pass_encoder = cx.graphics.create_encoder(cx.bump)?;

        let mut render_pass = render_pass_encoder.with_render_pass(
            &mut self.render_pass,
            &BasicRenderPass {
                color: swapchain_image.info().image.clone(),
                depth: Format::D16Unorm,
            },
            cx.graphics,
        )?;

        render_pass.bind_dynamic_graphics_pipeline(&mut self.pipeline, cx.graphics)?;

        let mut writes = BVec::new_in(cx.bump);
        for (e, (mesh, mat, global, renderable, scale)) in cx.world.query_mut::<(
            &Mesh,
            &Material,
            &Global3,
            &mut BasicRenderable,
            Option<&Scale>,
        )>() {
            if let Some(albedo) = mat.albedo_coverage.clone() {
                match scale {
                    Some(scale) => {
                        let mut m = na::Matrix4::<f32>::identity();
                        m[(0, 0)] = scale.0.x;
                        m[(1, 1)] = scale.0.y;
                        m[(2, 2)] = scale.0.z;

                        globals.transform = mat4_na_to_sierra(global.iso.to_homogeneous() * m);
                    }
                    None => {
                        globals.transform = mat4_na_to_sierra(global.iso.to_homogeneous());
                    }
                }

                let updated = renderable.descriptors.update(
                    &BasicDescriptors {
                        globals,
                        sampler: albedo.sampler,
                        albedo: albedo.image,
                    },
                    self.fence_index,
                    cx.graphics,
                    &mut writes,
                    &mut encoder,
                )?;

                render_pass.bind_graphics_descriptors(&self.pipeline_layout, updated);

                let drawn = mesh.draw(
                    0..1,
                    &[Position3d::layout(), Normal3d::layout(), UV::layout()],
                    &mut render_pass,
                    cx.bump,
                );

                if drawn {
                    tracing::info!("Mesh drawn");
                } else {
                    dbg!(mesh);
                    tracing::warn!("Mesh is not drawn");
                }
            }
        }

        cx.graphics.update_descriptor_sets(&writes, &[]);

        let fence = match &self.fences[self.fence_index] {
            Some(fence) => fence,
            None => self.fences[self.fence_index].get_or_insert(cx.graphics.create_fence()?),
        };

        drop(render_pass);

        cx.graphics.submit(
            &[(
                PipelineStageFlags::BOTTOM_OF_PIPE,
                &swapchain_image.info().wait,
            )],
            std::array::IntoIter::new([encoder.finish(), render_pass_encoder.finish()]),
            &[&swapchain_image.info().signal],
            Some(fence),
            cx.bump,
        );

        cx.graphics.present(swapchain_image)?;

        Ok(())
    }
}

impl Renderer for BasicRenderer {
    fn new(graphics: &mut Graphics) -> eyre::Result<Self> {
        let shader_module = graphics.create_shader_module(ShaderModuleInfo::wgsl(
            std::include_bytes!("basic.wgsl")
                .to_vec()
                .into_boxed_slice(),
        ))?;

        let pipeline_layout = BasicPipeline::layout(graphics)?;

        Ok(BasicRenderer {
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
                        stride: 8,
                    },
                ],
                vertex_attributes: vec![
                    VertexInputAttribute { location: 0, format: Format::RGB32Sfloat, binding: 0, offset: 0 },
                    VertexInputAttribute { location: 1, format: Format::RGB32Sfloat, binding: 1, offset: 0 },
                    VertexInputAttribute { location: 2, format: Format::RG32Sfloat, binding: 2, offset: 0 },
                ],
                vertex_shader: VertexShader::new(shader_module.clone(), "vs_main"),
                fragment_shader: Some(FragmentShader::new(shader_module.clone(), "fs_main")),
                layout: pipeline_layout.raw().clone(),
                depth_test: Some(DepthTest::LESS_WRITE),
            }),
            fences: [None, None, None],
            fence_index: 0,
            render_pass: BasicRenderPass::instance(),
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

#[inline(always)]
fn mat4_na_to_sierra(m: na::Matrix4<f32>) -> sierra::mat4<f32> {
    let array: [[f32; 4]; 4] = m.into();
    sierra::mat4::from(array)
}
