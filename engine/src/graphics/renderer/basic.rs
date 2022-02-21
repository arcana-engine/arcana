use edict::entity::EntityId;
use sierra::{
    descriptors, graphics_pipeline_desc, mat4, pipeline, shader_repr, vec4, DepthTest,
    DescriptorsInput, DynamicGraphicsPipeline, Encoder, Extent2d, Format, FragmentShader,
    ImageView, PipelineInput, RenderPassEncoder, Sampler, ShaderModuleInfo, VertexInputAttribute,
    VertexInputBinding, VertexInputRate, VertexShader,
};

use super::{mat4_na_to_sierra, DrawNode, RendererContext};
use crate::{
    assets::material::Material,
    camera::Camera3,
    graphics::{
        mesh::Mesh,
        vertex::{Normal3, Position3, VertexType as _, UV},
        Graphics, Scale,
    },
    scene::Global3,
};
pub struct BasicDraw {
    pipeline_layout: <BasicPipeline as PipelineInput>::Layout,
    pipeline: DynamicGraphicsPipeline,
}

#[shader_repr]
#[derive(Clone, Copy)]
struct Uniforms {
    albedo_factor: vec4,
    camera_view: mat4,
    camera_proj: mat4,
    transform: mat4,
    joints: [mat4; 128],
}

impl Default for Uniforms {
    fn default() -> Self {
        Uniforms {
            camera_view: mat4::default(),
            camera_proj: mat4::default(),
            transform: mat4::default(),
            joints: [mat4::default(); 128],
            albedo_factor: vec4::default(),
        }
    }
}

#[descriptors]
struct BasicDescriptors {
    #[sampler]
    #[stages(Fragment)]
    sampler: Sampler,

    #[image(sampled)]
    #[stages(Fragment)]
    albedo: ImageView,

    #[uniform]
    #[stages(Vertex, Fragment)]
    uniforms: Uniforms,
}

#[pipeline]
struct BasicPipeline {
    #[set]
    set: BasicDescriptors,
}

struct BasicRenderable {
    descriptors: <BasicDescriptors as DescriptorsInput>::Instance,
}

impl DrawNode for BasicDraw {
    fn draw<'a, 'b: 'a>(
        &'b mut self,
        cx: RendererContext<'a, 'b>,
        encoder: &mut Encoder<'a>,
        render_pass: &mut RenderPassEncoder<'_, 'b>,
        camera: EntityId,
        _viewport: Extent2d,
    ) -> eyre::Result<()> {
        let (global, camera) = cx.world.query_one::<(&Global3, &Camera3)>(&camera)?;

        let view = global.iso.inverse().to_homogeneous();
        let proj = camera.proj().to_homogeneous();

        let mut uniforms = Uniforms::default();
        uniforms.camera_view = mat4_na_to_sierra(view);
        uniforms.camera_proj = mat4_na_to_sierra(proj);

        let mut new_entities = Vec::new_in(&*cx.scope);

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
                .try_insert(
                    &e,
                    BasicRenderable {
                        descriptors: self.pipeline_layout.set.instance(),
                    },
                )
                .unwrap();
        }

        render_pass.bind_dynamic_graphics_pipeline(&mut self.pipeline, cx.graphics)?;

        let mut writes = Vec::new_in(&*cx.scope);

        let query = cx.world.query_mut::<(
            &Mesh,
            &Material,
            &Global3,
            &mut BasicRenderable,
            Option<&Scale>,
        )>();

        for (_, (mesh, mat, global, renderable, scale)) in query {
            uniforms.albedo_factor = mat.albedo_factor.into();

            if let Some(albedo) = mat.albedo.clone() {
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
                    &BasicDescriptors {
                        sampler: albedo.sampler,
                        albedo: albedo.image,
                        uniforms,
                    },
                    &*cx.graphics,
                    &mut *encoder,
                )?;

                render_pass.bind_graphics_descriptors(&self.pipeline_layout, updated);

                let mesh = cx.scope.to_scope(mesh.clone());
                let drawn = mesh.draw(
                    0..1,
                    &[Position3::layout(), Normal3::layout(), UV::layout()],
                    render_pass,
                );
                if !drawn {
                    tracing::warn!("Mesh is not drawn");
                }
            }
        }

        Ok(())
    }
}

impl BasicDraw {
    pub fn new(graphics: &mut Graphics) -> eyre::Result<Self> {
        let shader_module = graphics.create_shader_module(ShaderModuleInfo::wgsl(
            std::include_bytes!("basic.wgsl")
                .to_vec()
                .into_boxed_slice(),
        ))?;

        let pipeline_layout = BasicPipeline::layout(graphics)?;

        Ok(BasicDraw {
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
            pipeline_layout,
        })
    }
}
