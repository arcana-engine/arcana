//! Render pass to render EGUI
use std::mem;

use edict::entity::EntityId;
use palette::LinSrgba;
use sierra::{
    align_up, descriptors, graphics_pipeline_desc, pipeline, shader_repr, vec2, AccessFlags,
    Buffer, BufferMemoryBarrier, ComponentMapping, DynamicGraphicsPipeline, Encoder, Extent2d,
    FragmentShader, ImageInfo, ImageUsage, ImageView, ImageViewInfo, IndexType, Layout, Offset3d,
    PipelineInput, PipelineStageFlags, Rect2d, RenderPassEncoder, Sampler, ShaderModuleInfo, State,
    Swizzle, VertexInputRate, VertexShader,
};

use super::{DrawNode, RendererContext};
use crate::{
    egui::EguiResource,
    graphics::{
        vertex_layouts_for_pipeline, Graphics, Position2, UploadImage, VertexLocation, VertexType,
        UV,
    },
};
use egui::ClippedMesh;

#[shader_repr]
#[derive(Clone, Copy, Default)]
struct Uniforms {
    inv_dimensions: vec2,
}

#[descriptors]
struct EguiDescriptors {
    #[sampler]
    #[stages(Fragment)]
    sampler: Sampler,

    #[image(sampled)]
    #[stages(Fragment)]
    font_image: ImageView,

    #[uniform]
    #[stages(Vertex)]
    uniforms: Uniforms,
}

#[pipeline]
struct EguiPipeline {
    #[set]
    set: EguiDescriptors,
}

pub struct EguiDraw {
    pipeline: DynamicGraphicsPipeline,
    pipeline_layout: <EguiPipeline as PipelineInput>::Layout,
    descriptors: EguiDescriptors,
    set: EguiDescriptorsInstance,
    meshes: Buffer,
    font_image_version: Option<u64>,
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

        let dummy = graphics.create_image_static(
            sierra::ImageInfo {
                extent: sierra::ImageExtent::D2 {
                    width: 1,
                    height: 1,
                },
                format: sierra::Format::RGBA8Unorm,
                levels: 1,
                layers: 1,
                samples: sierra::Samples1,
                usage: sierra::ImageUsage::SAMPLED,
            },
            Layout::ShaderReadOnlyOptimal,
            &[255u8, 255, 255, 255],
            sierra::Format::RGBA8Unorm,
            4,
            1,
        )?;

        let dummy_texture = graphics.create_image_view(sierra::ImageViewInfo::new(dummy))?;
        let sampler = graphics.create_sampler(sierra::SamplerInfo::new())?;

        let meshes = graphics.create_buffer(sierra::BufferInfo {
            align: 255,
            size: 1 << 10,
            usage: sierra::BufferUsage::INDEX
                | sierra::BufferUsage::VERTEX
                | sierra::BufferUsage::TRANSFER_DST,
        })?;

        let set = pipeline_layout.set.instance();

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

            descriptors: EguiDescriptors {
                sampler,
                font_image: dummy_texture,
                uniforms: Uniforms::default(),
            },
            set,
            meshes,
            font_image_version: None,
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
        viewport: Extent2d,
    ) -> eyre::Result<()> {
        let res = match cx.res.get_mut::<EguiResource>() {
            None => return Ok(()),
            Some(ctx) => ctx,
        };

        let scale_factor = res.scale_factor();

        self.descriptors.uniforms.inv_dimensions = vec2::from([
            2.0 * scale_factor / viewport.width as f32,
            -2.0 * scale_factor / viewport.height as f32,
        ]);

        let font_image = res.font_image();
        if self.font_image_version != Some(font_image.version) {
            self.font_image_version = Some(font_image.version);

            let font_image_info = self.descriptors.font_image.info().image.info();
            let font_image_info_extent = font_image_info.extent.into_2d();

            let mut old_layout = Some(sierra::Layout::ShaderReadOnlyOptimal);

            if font_image_info_extent.width != font_image.width as u32
                || font_image_info_extent.height != font_image.height as u32
            {
                let new_font_image = cx.graphics.create_image(ImageInfo {
                    extent: Extent2d {
                        width: font_image.width as u32,
                        height: font_image.height as u32,
                    }
                    .into(),
                    format: sierra::Format::R8Unorm,
                    levels: 1,
                    layers: 1,
                    samples: sierra::Samples1,
                    usage: ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
                })?;

                let new_font_image_view = cx.graphics.create_image_view(ImageViewInfo {
                    mapping: ComponentMapping {
                        r: Swizzle::One,
                        g: Swizzle::One,
                        b: Swizzle::One,
                        a: Swizzle::R,
                    },
                    ..ImageViewInfo::new(new_font_image)
                })?;

                self.descriptors.font_image = new_font_image_view;
                old_layout = None;
            }

            cx.graphics.upload_image_with(
                UploadImage {
                    image: &self.descriptors.font_image.info().image,
                    offset: Offset3d::ZERO,
                    extent: Extent2d {
                        width: font_image.width as u32,
                        height: font_image.height as u32,
                    }
                    .into_3d(),
                    layers: sierra::SubresourceLayers::color(0, 0..1),
                    old_layout,
                    new_layout: sierra::Layout::ShaderReadOnlyOptimal,
                    old_access: sierra::AccessFlags::SHADER_READ,
                    new_access: sierra::AccessFlags::SHADER_READ,
                    format: sierra::Format::R8Unorm,
                    row_length: 0,
                    image_height: 0,
                },
                &font_image.pixels[..],
                encoder,
            )?;
        }

        let updated = self.set.update(&self.descriptors, cx.graphics, encoder)?;

        render_pass.bind_dynamic_graphics_pipeline(&mut self.pipeline, cx.graphics)?;
        render_pass.bind_graphics_descriptors(&self.pipeline_layout, updated);

        let mut buffer_offset = 0;

        for ClippedMesh(rect, mesh) in res.meshes() {
            render_pass.set_scissor(Rect2d {
                offset: sierra::Offset2d {
                    x: (rect.min.x * scale_factor) as i32,
                    y: (rect.min.y * scale_factor) as i32,
                },
                extent: Extent2d {
                    width: ((rect.max.x - rect.min.x) * scale_factor) as u32,
                    height: ((rect.max.y - rect.min.y) * scale_factor) as u32,
                },
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
                        PipelineStageFlags::TRANSFER,
                        PipelineStageFlags::VERTEX_INPUT,
                        &[BufferMemoryBarrier {
                            buffer: &self.meshes,
                            offset: 0,
                            size: buffer_offset,
                            old_access: AccessFlags::TRANSFER_WRITE,
                            new_access: AccessFlags::INDEX_READ
                                | AccessFlags::VERTEX_ATTRIBUTE_READ,
                            family_transfer: None,
                        }],
                    );
                }
            }
            buffer_offset = vertices_offset + vertices_size;

            encoder.update_buffer(&self.meshes, indices_offset, &*mesh.indices);
            encoder.update_buffer(&self.meshes, vertices_offset, &*mesh.vertices);

            render_pass.bind_index_buffer(&self.meshes, indices_offset, IndexType::U32);
            render_pass.bind_vertex_buffers(0, &[(&self.meshes, vertices_offset)]);
            render_pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        }

        encoder.buffer_barriers(
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::VERTEX_INPUT,
            &[BufferMemoryBarrier {
                buffer: &self.meshes,
                offset: 0,
                size: buffer_offset,
                old_access: AccessFlags::TRANSFER_WRITE,
                new_access: AccessFlags::INDEX_READ | AccessFlags::VERTEX_ATTRIBUTE_READ,
                family_transfer: None,
            }],
        );

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
