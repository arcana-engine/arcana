use std::{collections::HashMap, convert::TryFrom, mem::size_of_val};

use hecs::Entity;
use palette::LinSrgba;
use sierra::{
    align_up, descriptors, graphics_pipeline_desc, pipeline, shader_repr, AccessFlags, Buffer,
    BufferImageCopy, BufferInfo, BufferUsage, DynamicGraphicsPipeline, Encoder, Extent2d, Extent3d,
    Format, FragmentShader, ImageExtent, ImageInfo, ImageUsage, ImageView, ImageViewInfo,
    IndexType, Layout, LayoutTransition, Offset3d, PipelineInput, PipelineStage,
    PipelineStageFlags, RenderPassEncoder, Sampler, Samples::Samples1, ShaderModuleInfo,
    SubresourceLayers, Swizzle, VertexInputRate, VertexShader,
};

use super::{DrawNode, RendererContext};
use crate::{
    assets::font::Font,
    graphics::{
        vertex_layouts_for_pipeline, Graphics, ImageAsset, Position2, SparseDescriptors,
        VertexLocation, VertexType, UV,
    },
    task::with_async_task_context,
};
use sigils::Ui;

#[shader_repr]
#[derive(Clone, Copy)]
struct Uniforms {
    albedo: u32,
}

impl Default for Uniforms {
    fn default() -> Self {
        Uniforms { albedo: u32::MAX }
    }
}

#[descriptors]
struct SigilsDescriptors {
    #[sampler]
    #[stages(Fragment)]
    sampler: Sampler,

    #[sampled_image]
    #[stages(Fragment)]
    textures: [ImageView; 128],
}

#[pipeline]
struct SigilsPipeline {
    #[set]
    set: SigilsDescriptors,
}

pub struct SigilsDraw {
    pipeline: DynamicGraphicsPipeline,
    pipeline_layout: <SigilsPipeline as PipelineInput>::Layout,
    descriptors: SigilsDescriptors,
    set: SigilsDescriptorsInstance,
    textures: SparseDescriptors<ImageView>,
    meshes: Buffer,
    font_upload_buffer: Option<Buffer>,
    font_atlas_view: Option<ImageView>,
}

struct SigilsDrawAssets {
    images: HashMap<Uuid, ImageView>,
    fonts: HashMap<Uuid, rusttype::Font<'static>>,
}

impl SigilsDraw {
    pub fn new(graphics: &mut Graphics) -> eyre::Result<Self> {
        let vert_module = graphics.create_shader_module(ShaderModuleInfo::spirv(
            std::include_bytes!("sigils.vert.spv")
                .to_vec()
                .into_boxed_slice(),
        ))?;

        let frag_module = graphics.create_shader_module(ShaderModuleInfo::spirv(
            std::include_bytes!("sigils.frag.spv")
                .to_vec()
                .into_boxed_slice(),
        ))?;

        let pipeline_layout = SigilsPipeline::layout(graphics)?;

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
            4,
            1,
            &[255u8, 255, 255, 255],
        )?;

        let dummy = graphics.create_image_view(sierra::ImageViewInfo::new(dummy))?;
        let textures = (0..128).map(|_| dummy.clone()).collect::<Vec<_>>();
        let textures = <[ImageView; 128]>::try_from(textures).unwrap();

        let sampler = graphics.create_sampler(sierra::SamplerInfo::new())?;

        let meshes = graphics.create_buffer(sierra::BufferInfo {
            align: 255,
            size: 1 << 13,
            usage: sierra::BufferUsage::INDEX
                | sierra::BufferUsage::VERTEX
                | sierra::BufferUsage::TRANSFER_DST,
        })?;

        let set = pipeline_layout.set.instance();

        let (vertex_bindings, vertex_attributes) = vertex_layouts_for_pipeline(&[Vertex::layout()]);

        Ok(SigilsDraw {
            pipeline: DynamicGraphicsPipeline::new(graphics_pipeline_desc! {
                vertex_bindings,
                vertex_attributes,
                vertex_shader: VertexShader::new(vert_module.clone(), "main"),
                fragment_shader: Some(FragmentShader::new(frag_module.clone(), "main")),
                layout: pipeline_layout.raw().clone(),
                depth_test: None,
            }),
            pipeline_layout,

            descriptors: SigilsDescriptors { sampler, textures },
            set,
            textures: SparseDescriptors::new(),
            meshes,
            font_upload_buffer: None,
            font_atlas_view: None,
        })
    }
}

impl DrawNode for SigilsDraw {
    fn draw<'a, 'b: 'a>(
        &'b mut self,
        cx: RendererContext<'a, 'b>,
        fence_index: usize,
        encoder: &mut Encoder<'a>,
        render_pass: &mut RenderPassEncoder<'_, 'b>,
        _camera: Entity,
        viewport: Extent2d,
    ) -> eyre::Result<()> {
        cx.res.with(|| SigilsDrawAssets {
            images: HashMap::new(),
            fonts: HashMap::new(),
        });

        let (ui, assets) = match cx.res.get_two_mut::<Ui, SigilsDrawAssets>() {
            None => return Ok(()),
            Some(pair) => pair,
        };

        let mut missing_images = Vec::new();
        let mut missing_fonts = Vec::new();

        let mut writes = Vec::new_in(&*cx.scope);
        let mut indices = Vec::new_in(&*cx.scope);
        let mut vertices = Vec::new_in(&*cx.scope);

        // Generate meshes for UI
        let (meshes, glyphs) = ui.render(
            |f| match assets.fonts.get(f) {
                None => {
                    missing_fonts.push(*f);
                    None
                }
                Some(f) => Some(f),
            },
            cx.scope,
        );

        if let Some(font_atlas) = font_atlas {
            let update_data = font_atlas.update_data;
            let updates = font_atlas.updates;

            if !updates.is_empty() {
                debug_assert!(!update_data.is_empty());

                match &mut self.font_atlas_view {
                    None => {
                        let font_image = cx.graphics.create_image(ImageInfo {
                            extent: ImageExtent::D2 {
                                width: font_atlas.extent.x,
                                height: font_atlas.extent.y,
                            },
                            format: Format::R8Unorm,
                            levels: 1,
                            layers: 1,
                            samples: Samples1,
                            usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                        })?;

                        let mut view_info = ImageViewInfo::new(font_image.clone());
                        view_info.mapping.r = Swizzle::One;
                        view_info.mapping.g = Swizzle::One;
                        view_info.mapping.b = Swizzle::One;
                        view_info.mapping.a = Swizzle::R;

                        let font_view = cx.graphics.create_image_view(view_info)?;
                        self.font_atlas_view = Some(font_view);

                        encoder.image_barriers(
                            PipelineStageFlags::BOTTOM_OF_PIPE,
                            PipelineStageFlags::TRANSFER,
                            &*cx.scope.to_scope([LayoutTransition::initialize_whole(
                                cx.scope.to_scope(font_image),
                                AccessFlags::empty(),
                                Layout::TransferDstOptimal,
                            )
                            .into()]),
                        );
                    }
                    Some(font_atlas_view) => {
                        let font_atlas_extent =
                            font_atlas_view.info().image.info().extent.into_2d();

                        if font_atlas_extent.width != font_atlas.extent.x
                            || font_atlas_extent.height != font_atlas.extent.y
                        {
                            let font_image = cx.graphics.create_image(ImageInfo {
                                extent: ImageExtent::D2 {
                                    width: font_atlas.extent.x,
                                    height: font_atlas.extent.y,
                                },
                                format: Format::R8Unorm,
                                levels: 1,
                                layers: 1,
                                samples: Samples1,
                                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                            })?;

                            let mut view_info = ImageViewInfo::new(font_image);
                            view_info.mapping.r = Swizzle::One;
                            view_info.mapping.g = Swizzle::One;
                            view_info.mapping.b = Swizzle::One;
                            view_info.mapping.a = Swizzle::R;

                            let font_view = cx.graphics.create_image_view(view_info)?;
                            self.font_atlas_view = Some(font_view);

                            let font_image = &self.font_atlas_view.as_ref().unwrap().info().image;

                            encoder.image_barriers(
                                PipelineStageFlags::BOTTOM_OF_PIPE,
                                PipelineStageFlags::TRANSFER,
                                &*cx.scope.to_scope([LayoutTransition::initialize_whole(
                                    font_image,
                                    AccessFlags::empty(),
                                    Layout::TransferDstOptimal,
                                )
                                .into()]),
                            );
                        } else {
                            let font_image = &self.font_atlas_view.as_ref().unwrap().info().image;

                            encoder.image_barriers(
                                PipelineStageFlags::FRAGMENT_SHADER,
                                PipelineStageFlags::TRANSFER,
                                &*cx.scope.to_scope([LayoutTransition::transition_whole(
                                    font_image,
                                    AccessFlags::SHADER_READ..AccessFlags::TRANSFER_WRITE,
                                    Layout::ShaderReadOnlyOptimal..Layout::TransferDstOptimal,
                                )
                                .into()]),
                            );
                        }
                    }
                }

                let required_upload_size = align_up(3, update_data.len() as u64).unwrap();

                match &mut self.font_upload_buffer {
                    None => {
                        let font_upload_buffer = cx.graphics.create_buffer(BufferInfo {
                            size: required_upload_size,
                            align: 255,
                            usage: BufferUsage::TRANSFER_SRC | BufferUsage::TRANSFER_DST,
                        })?;
                        self.font_upload_buffer = Some(font_upload_buffer);
                    }
                    Some(font_upload_buffer) => {
                        if font_upload_buffer.info().size < required_upload_size {
                            let font_upload_buffer = cx.graphics.create_buffer(BufferInfo {
                                size: required_upload_size,
                                align: 255,
                                usage: BufferUsage::TRANSFER_SRC | BufferUsage::TRANSFER_DST,
                            })?;

                            self.font_upload_buffer = Some(font_upload_buffer);
                        }
                    }
                }

                let font_atlas_image = &self.font_atlas_view.as_ref().unwrap().info().image;
                let font_upload_buffer = self.font_upload_buffer.as_ref().unwrap();

                encoder.memory_barrier(
                    PipelineStageFlags::TRANSFER,
                    AccessFlags::TRANSFER_READ,
                    PipelineStageFlags::TRANSFER,
                    AccessFlags::TRANSFER_WRITE,
                );

                if update_data.len() & 3 == 0 {
                    cx.graphics
                        .upload_buffer_with(font_upload_buffer, 0, update_data, encoder)?;
                } else if update_data.len() <= 128 {
                    // Copy whole data and upload from copy.
                    let mut data_copy = [0; 128];
                    data_copy[..update_data.len()].copy_from_slice(update_data);
                    encoder.update_buffer(font_upload_buffer, 0, cx.scope.to_scope(data_copy));
                } else {
                    let head = update_data.len() & !3;
                    let tail = update_data.len() & 3;

                    // Upload truncated
                    cx.graphics.upload_buffer_with(
                        font_upload_buffer,
                        0,
                        &update_data[..head],
                        encoder,
                    )?;

                    // Upload the rest
                    let mut data_copy = [0; 4];
                    data_copy[..tail].copy_from_slice(&update_data[head..]);
                    encoder.update_buffer(
                        font_upload_buffer,
                        head as u64,
                        cx.scope.to_scope(data_copy),
                    );
                }

                encoder.memory_barrier(
                    PipelineStageFlags::TRANSFER,
                    AccessFlags::TRANSFER_WRITE,
                    PipelineStageFlags::TRANSFER,
                    AccessFlags::TRANSFER_READ,
                );

                // Upload font atlas image interleaving with buffer data upload.
                for update in updates {
                    // Copy from font upload buffer into font atlas image.
                    encoder.copy_buffer_to_image(
                        font_upload_buffer,
                        font_atlas_image,
                        Layout::TransferDstOptimal,
                        &*cx.scope.to_scope([BufferImageCopy {
                            buffer_offset: update.src_start as u64,
                            buffer_row_length: update.dst_extent.x,
                            buffer_image_height: 0,
                            image_subresource: SubresourceLayers::all_layers(
                                font_atlas_image.info(),
                                0,
                            ),
                            image_offset: Offset3d {
                                x: update.dst_offset.x as i32,
                                y: update.dst_offset.y as i32,
                                z: 0,
                            },
                            image_extent: Extent3d {
                                width: update.dst_extent.x,
                                height: update.dst_extent.y,
                                depth: 1,
                            },
                        }]),
                    );
                }

                encoder.image_barriers(
                    PipelineStageFlags::TRANSFER,
                    PipelineStageFlags::FRAGMENT_SHADER,
                    &*cx.scope.to_scope([LayoutTransition::transition_whole(
                        font_atlas_image,
                        AccessFlags::TRANSFER_WRITE..AccessFlags::SHADER_READ,
                        Layout::TransferDstOptimal..Layout::ShaderReadOnlyOptimal,
                    )
                    .into()]),
                );
            }
        }

        struct Draw {
            start: u32,
            end: u32,
        }

        let font_atlas_view = self.font_atlas_view.as_ref();
        let textures = &mut self.textures;
        let descriptors = &mut self.descriptors;

        // Generate draws
        let draws = cx
            .scope
            .to_scope_from_iter(meshes.iter().filter_map(|mesh| {
                let albedo = match mesh.texture {
                    Some(sigils::Texture::Font) => {
                        let font_atlas_view = font_atlas_view?;
                        let (index, new) = textures.index(font_atlas_view.clone());
                        if new {
                            descriptors.textures[index as usize] = font_atlas_view.clone();
                        }
                        index
                    }
                    Some(sigils::Texture::Texture(texture)) => match assets.images.get(texture) {
                        None => {
                            missing_images.push(*texture);
                            return None;
                        }
                        Some(image) => {
                            let (index, new) = textures.index(image.clone());
                            if new {
                                descriptors.textures[index as usize] = image.clone();
                            }
                            index
                        }
                    },
                    None => u32::MAX,
                };

                let vertex_offset = vertices.len() as u32;

                let start = indices.len() as u32;
                indices.extend(mesh.indices.iter().map(|i| i + vertex_offset));
                let end = indices.len() as u32;

                vertices.extend(mesh.vertices.iter().map(|v| Vertex {
                    pos: Position2([
                        v.pos.x * 2.0 / (viewport.width as f32) - 1.0,
                        v.pos.y * 2.0 / (viewport.height as f32) - 1.0,
                    ]),
                    uv: UV([v.uv.x, v.uv.y]),
                    albedo,
                    albedo_factor: LinSrgba::new(v.color[0], v.color[1], v.color[2], v.color[3]),
                }));

                Some(Draw { start, end })
            }));

        // "leak" values. Those vectors are allocated from scope so memory will be reclaimed anyway.
        // `Drop` would be skipped, but here are `Copy` types.
        let indices = &*indices.leak();
        let vertices = &*vertices.leak();

        let total_size = size_of_val(indices) + size_of_val(vertices);
        if self.meshes.info().size < total_size as u64 {
            self.meshes = cx.graphics.create_buffer(sierra::BufferInfo {
                align: 255,
                size: (self.meshes.info().size + 1)
                    .next_power_of_two()
                    .max(total_size as u64),
                usage: sierra::BufferUsage::INDEX
                    | sierra::BufferUsage::VERTEX
                    | sierra::BufferUsage::TRANSFER_DST,
            })?;
        }

        render_pass.bind_dynamic_graphics_pipeline(&mut self.pipeline, cx.graphics)?;

        let updated = self.set.update(
            &self.descriptors,
            fence_index,
            cx.graphics,
            &mut writes,
            encoder,
        )?;

        cx.graphics.update_descriptor_sets(&writes, &[]);

        render_pass.bind_graphics_descriptors(&self.pipeline_layout, updated);

        cx.graphics
            .upload_buffer_with(&self.meshes, 0, indices, encoder)?;

        cx.graphics.upload_buffer_with(
            &self.meshes,
            size_of_val(indices) as u64,
            vertices,
            encoder,
        )?;

        encoder.memory_barrier(
            PipelineStageFlags::TRANSFER,
            AccessFlags::TRANSFER_WRITE,
            PipelineStageFlags::VERTEX_INPUT,
            AccessFlags::INDEX_READ | AccessFlags::VERTEX_ATTRIBUTE_READ,
        );

        render_pass.bind_index_buffer(&self.meshes, 0, IndexType::U32);

        let vertex_buffers = render_pass
            .scope()
            .to_scope([(&self.meshes, size_of_val(indices) as u64)]);

        render_pass.bind_vertex_buffers(0, vertex_buffers);

        for draw in draws {
            render_pass.draw_indexed(draw.start..draw.end, 0, 0..1);
        }

        let loader = cx.loader;

        let missing_images: Vec<_> = missing_images
            .into_iter()
            .map(|uuid| (uuid, loader.load::<ImageAsset>(&uuid)))
            .collect();

        let missing_fonts: Vec<_> = missing_fonts
            .into_iter()
            .map(|uuid| (uuid, loader.load::<FontAsset>(&uuid)))
            .collect();

        if !missing_images.is_empty() || !missing_fonts.is_empty() {
            cx.spawner.spawn(async move {
                let mut images = Vec::new();

                for (uuid, image) in missing_images {
                    images.push((uuid, image.await));
                }

                let mut fonts = Vec::new();

                for (uuid, font) in missing_fonts {
                    fonts.push((uuid, font.await));
                }

                with_async_task_context(move |cx| {
                    let assets = cx.res.get_mut::<SigilsDrawAssets>().unwrap();

                    for (uuid, mut image) in images {
                        match image.get(cx.graphics) {
                            Ok(image) => {
                                assets.images.insert(uuid, image.clone().into_inner());
                            }
                            Err(err) => {
                                tracing::error!("Failed to load UI texture {}: {:#?}", uuid, err);
                            }
                        }
                    }

                    for (uuid, mut font) in fonts {
                        match font.get(&mut ()) {
                            Ok(font) => {
                                assets.fonts.insert(uuid, font.clone().into_inner());
                            }
                            Err(err) => {
                                tracing::error!("Failed to load UI font {}: {:#?}", uuid, err);
                            }
                        }
                    }
                });

                Ok(())
            })
        }

        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Vertex {
    pos: Position2,
    uv: UV,
    albedo: u32,
    albedo_factor: LinSrgba<f32>,
}

unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

impl VertexType for Vertex {
    const LOCATIONS: &'static [VertexLocation] = {
        let mut offset = 0;
        &[
            vertex_location!(offset, Position2),
            vertex_location!(offset, UV),
            vertex_location!(offset, u32 as "Albedo"),
            vertex_location!(offset, LinSrgba<f32>),
        ]
    };
    const RATE: VertexInputRate = VertexInputRate::Vertex;
}
