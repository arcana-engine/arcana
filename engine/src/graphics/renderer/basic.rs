use edict::entity::EntityId;
use sierra::{
    graphics_pipeline_desc, mat4, vec4, DepthTest, Descriptors, DynamicGraphicsPipeline, Encoder,
    Extent2, FragmentShader, ImageView, PipelineInput, RenderPassEncoder, Sampler,
    ShaderModuleInfo, ShaderRepr, VertexShader,
};

use super::{mat4_na_to_sierra, DrawNode, RendererContext};
use crate::{
    camera::Camera3,
    graphics::{
        material::Material,
        mesh::Mesh,
        vertex::{Normal3, Position3, VertexType as _, UV, V3},
        vertex_layouts_for_pipeline, Graphics, Scale,
    },
    scene::Global3,
};
pub struct BasicDraw {
    pipeline_layout: <BasicPipeline as PipelineInput>::Layout,
    pipeline: DynamicGraphicsPipeline,
}

#[derive(Clone, Copy, ShaderRepr)]
#[sierra(std140)]
struct Uniforms {
    albedo_factor: vec4,
    camera_view: mat4,
    camera_proj: mat4,
    transform: mat4,
    joints: [mat4; 128],
}

impl Default for Uniforms {
    #[inline]
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

#[derive(Descriptors)]
struct BasicDescriptors {
    #[sierra(sampler, fragment)]
    sampler: Sampler,

    #[sierra(image(sampled), fragment)]
    albedo: ImageView,

    #[sierra(uniform, stages(vertex, fragment))]
    uniforms: Uniforms,
}

#[allow(unused)]
#[derive(PipelineInput)]
struct BasicPipeline {
    #[sierra(set)]
    set: BasicDescriptors,
}

struct BasicRenderable {
    descriptors: <BasicDescriptors as Descriptors>::Instance,
}

impl DrawNode for BasicDraw {
    fn draw<'a, 'b: 'a>(
        &'b mut self,
        cx: RendererContext<'a, 'b>,
        encoder: &mut Encoder<'a>,
        render_pass: &mut RenderPassEncoder<'_, 'b>,
        camera: EntityId,
        _viewport: Extent2,
    ) -> eyre::Result<()> {
        let (global, camera) = cx.world.query_one::<(&Global3, &Camera3)>(&camera)?;

        let view = global.iso.inverse().to_homogeneous();
        let proj = camera.proj().to_homogeneous();

        let mut uniforms = Uniforms {
            camera_view: mat4_na_to_sierra(view),
            camera_proj: mat4_na_to_sierra(proj),
            ..Uniforms::default()
        };

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

        if !new_entities.is_empty() {
            tracing::info!("{} new meshes", new_entities.len());
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

        let query = cx.world.query_mut::<(
            &Mesh,
            &Material,
            &Global3,
            &mut BasicRenderable,
            Option<&Scale>,
        )>();

        // let mut drawn_count = 0;
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

                let drawn = mesh.draw(0..1, &[V3::<Position3, Normal3, UV>::layout()], render_pass);
                if !drawn {
                    tracing::warn!("Mesh is not drawn");
                } else {
                    // drawn_count += 1;
                }
            }
        }

        // tracing::info!("Meshes drawn {}", drawn_count);

        Ok(())
    }
}

impl BasicDraw {
    pub fn new(graphics: &Graphics) -> eyre::Result<Self> {
        let shader_module = graphics.create_shader_module(ShaderModuleInfo::wgsl(
            std::include_bytes!("basic.wgsl")
                .to_vec()
                .into_boxed_slice(),
        ))?;

        let pipeline_layout = BasicPipeline::layout(graphics)?;

        let (vertex_bindings, vertex_attributes) =
            vertex_layouts_for_pipeline(&[V3::<Position3, Normal3, UV>::layout()]);

        Ok(BasicDraw {
            pipeline: DynamicGraphicsPipeline::new(graphics_pipeline_desc! {
                vertex_bindings,
                vertex_attributes,
                vertex_shader: VertexShader::new(shader_module.clone(), "vs_main"),
                fragment_shader: Some(FragmentShader::new(shader_module, "fs_main")),
                layout: pipeline_layout.raw().clone(),
                depth_test: Some(DepthTest::LESS_WRITE),
            }),
            pipeline_layout,
        })
    }
}
