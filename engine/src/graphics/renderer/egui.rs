//! Render pass to render EGUI
use std::mem;

use edict::entity::EntityId;
use hashbrown::hash_map::{Entry, HashMap};
use palette::LinSrgba;
use sierra::{
    align_up, graphics_pipeline_desc, vec2, Access, Buffer, BufferMemoryBarrier, Descriptors,
    DynamicGraphicsPipeline, Encoder, Extent2, FragmentShader, ImageView, IndexType, Offset2,
    PipelineInput, PipelineStages, Rect, RenderPassEncoder, Sampler, ShaderModuleInfo, ShaderRepr,
    State, VertexInputRate, VertexShader,
};

use super::{DrawNode, RendererContext};
use crate::{
    egui::EguiResource,
    graphics::{vertex_layouts_for_pipeline, Graphics, Position2, VertexLocation, VertexType, UV},
};
use egui::{epaint::Primitive, ClippedPrimitive, TextureId};

#[derive(Clone, Copy, Default, ShaderRepr)]
#[sierra(std140)]
struct Uniforms {
    inv_dimensions: vec2,
}

#[derive(Descriptors)]
struct TextureDescriptor {
    #[sierra(image(sampled), fragment)]
    texture: ImageView,
}

#[derive(Descriptors)]
struct SamplerUniforms {
    #[sierra(sampler, fragment)]
    sampler: Sampler,

    #[sierra(uniform, vertex)]
    uniforms: Uniforms,
}

#[derive(PipelineInput)]
struct EguiPipeline {
    #[sierra(set)]
    #[allow(unused)]
    sampler_uniforms: SamplerUniforms,

    #[sierra(set)]
    #[allow(unused)]
    texture: TextureDescriptor,
}

pub struct EguiDraw {
    pipeline: DynamicGraphicsPipeline,
    pipeline_layout: EguiPipelineLayout,
    sampler_uniforms: SamplerUniforms,
    sampler_uniforms_set: SamplerUniformsInstance,
    meshes: Buffer,
    textures: HashMap<TextureId, TextureDescriptorInstance>,
}

impl EguiDraw {
    pub fn new(graphics: &mut Graphics) -> eyre::Result<Self> {
        let vert_module = graphics.create_shader_module(ShaderModuleInfo::glsl(
            std::include_bytes!("egui.vert").to_vec().into_boxed_slice(),
            sierra::ShaderStage::Vertex,
        ))?;

        let frag_module = graphics.create_shader_module(ShaderModuleInfo::glsl(
            std::include_bytes!("egui.frag").to_vec().into_boxed_slice(),
            sierra::ShaderStage::Fragment,
        ))?;

        let pipeline_layout = EguiPipeline::layout(graphics)?;

        let sampler = graphics.create_sampler(sierra::SamplerInfo::new())?;

        let meshes = graphics.create_buffer(sierra::BufferInfo {
            align: 255,
            size: 1 << 10,
            usage: sierra::BufferUsage::INDEX
                | sierra::BufferUsage::VERTEX
                | sierra::BufferUsage::TRANSFER_DST,
        })?;

        let sampler_uniforms_set = pipeline_layout.sampler_uniforms.instance();

        let (vertex_bindings, vertex_attributes) =
            vertex_layouts_for_pipeline(&[egui::epaint::Vertex::layout()]);

        Ok(EguiDraw {
            pipeline: DynamicGraphicsPipeline::new(graphics_pipeline_desc! {
                vertex_bindings,
                vertex_attributes,
                vertex_shader: VertexShader::new(vert_module, "main"),
                fragment_shader: Some(FragmentShader::new(frag_module, "main")),
                layout: pipeline_layout.raw().clone(),
                depth_test: None,
                scissor: State::Dynamic,
            }),
            pipeline_layout,

            sampler_uniforms: SamplerUniforms {
                sampler,
                uniforms: Uniforms::default(),
            },
            sampler_uniforms_set,
            meshes,
            textures: HashMap::new(),
        })
    }
}

impl DrawNode for EguiDraw {
    fn draw<'a, 'b: 'a>(
        &'b mut self,
        cx: RendererContext<'a, 'b>,
        encoder: &mut Encoder<'a>,
        render_pass: &mut RenderPassEncoder<'_, 'b>,
        _camera: EntityId,
        viewport: Extent2,
    ) -> eyre::Result<()> {
        let res = match cx.res.get_mut::<EguiResource>() {
            None => return Ok(()),
            Some(ctx) => ctx,
        };

        let scale_factor = res.scale_factor();

        self.sampler_uniforms.uniforms.inv_dimensions = vec2::from([
            2.0 * scale_factor / viewport.width as f32,
            -2.0 * scale_factor / viewport.height as f32,
        ]);

        res.update_egui_textures(encoder, cx.graphics)?;

        let updated =
            self.sampler_uniforms_set
                .update(&self.sampler_uniforms, cx.graphics, encoder)?;

        render_pass.bind_dynamic_graphics_pipeline(&mut self.pipeline, cx.graphics)?;
        render_pass.bind_graphics_descriptors(&self.pipeline_layout, updated);

        let mut buffer_offset = 0;

        for clipped_primitives in res.primitives() {
            let (clip_rect, mesh) = match clipped_primitives {
                ClippedPrimitive {
                    clip_rect,
                    primitive: Primitive::Mesh(mesh),
                } => (clip_rect, mesh),
                _ => continue,
            };

            render_pass.set_scissor(Rect {
                offset: Offset2::new(
                    (clip_rect.min.x * scale_factor) as i32,
                    (clip_rect.min.y * scale_factor) as i32,
                ),
                extent: Extent2::new(
                    ((clip_rect.max.x - clip_rect.min.x) * scale_factor) as u32,
                    ((clip_rect.max.y - clip_rect.min.y) * scale_factor) as u32,
                ),
            });

            let mut indices_offset = align_up(3, buffer_offset).unwrap();
            let indices_size = mem::size_of_val(&*mesh.indices) as u64;
            let mut vertices_offset = align_up(3, indices_offset + indices_size).unwrap();
            let vertices_size = mem::size_of_val(&*mesh.vertices) as u64;

            if vertices_offset + vertices_size > self.meshes.info().size {
                // Doesn't fit.
                // Make new buffer.

                let new_size = (vertices_offset + vertices_size).max(self.meshes.info().size * 2);

                self.meshes = cx.graphics.create_buffer(sierra::BufferInfo {
                    align: 255,
                    size: new_size,
                    usage: sierra::BufferUsage::INDEX
                        | sierra::BufferUsage::VERTEX
                        | sierra::BufferUsage::TRANSFER_DST,
                })?;

                indices_offset = 0;
                vertices_offset = align_up(3, indices_offset + indices_size).unwrap();

                if buffer_offset > 0 {
                    encoder.buffer_barriers(
                        PipelineStages::TRANSFER,
                        PipelineStages::VERTEX_INPUT,
                        &[BufferMemoryBarrier {
                            buffer: &self.meshes,
                            offset: 0,
                            size: buffer_offset,
                            old_access: Access::TRANSFER_WRITE,
                            new_access: Access::INDEX_READ | Access::VERTEX_ATTRIBUTE_READ,
                            family_transfer: None,
                        }],
                    );
                }
            }
            buffer_offset = vertices_offset + vertices_size;

            encoder.update_buffer(&self.meshes, indices_offset, &*mesh.indices);
            encoder.update_buffer(&self.meshes, vertices_offset, &*mesh.vertices);

            let texture_set = match self.textures.entry(mesh.texture_id) {
                Entry::Vacant(entry) => {
                    let texture_set = self.pipeline_layout.texture.instance();

                    entry.insert(texture_set)
                }
                Entry::Occupied(entry) => entry.into_mut(),
            };

            let view = match res.get_texture(mesh.texture_id) {
                None => {
                    tracing::error!("Missing texture '{:?}'", mesh.texture_id);
                    continue;
                }
                Some(view) => view,
            };
            let updated = texture_set.update(
                &TextureDescriptor {
                    texture: view.clone(),
                },
                cx.graphics,
                encoder,
            )?;

            render_pass.bind_graphics_descriptors(&self.pipeline_layout, updated);
            render_pass.bind_index_buffer(&self.meshes, indices_offset, IndexType::U32);
            render_pass.bind_vertex_buffers(0, &[(&self.meshes, vertices_offset)]);
            render_pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        }

        encoder.buffer_barriers(
            PipelineStages::TRANSFER,
            PipelineStages::VERTEX_INPUT,
            &[BufferMemoryBarrier {
                buffer: &self.meshes,
                offset: 0,
                size: buffer_offset,
                old_access: Access::TRANSFER_WRITE,
                new_access: Access::INDEX_READ | Access::VERTEX_ATTRIBUTE_READ,
                family_transfer: None,
            }],
        );

        for id in res.free_textures() {
            self.textures.remove(&id);
        }

        Ok(())
    }
}

// #[repr(C)]
// #[derive(Clone, Copy, Debug, Default, PartialEq)]
// struct Vertex {
//     pos: Position2,
//     uv: UV,
//     albedo_factor: LinSrgba<f32>,
// }

// unsafe impl bytemuck::Zeroable for Vertex {}
// unsafe impl bytemuck::Pod for Vertex {}

impl VertexType for egui::epaint::Vertex {
    const LOCATIONS: &'static [VertexLocation] = {
        let mut offset = 0;
        let pos = vertex_location!(offset, Position2);
        let uv = vertex_location!(offset, UV);
        let color = vertex_location!(offset, LinSrgba<u8>);
        &[pos, uv, color]
    };
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}
