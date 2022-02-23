use std::{convert::TryFrom, mem::size_of, ops::Range};

use edict::entity::EntityId;
use palette::LinSrgba;
use sierra::{
    descriptors, graphics_pipeline_desc, mat3, pipeline, shader_repr, AccessFlags, Buffer,
    DepthTest, DynamicGraphicsPipeline, Encoder, Extent2d, FragmentShader, ImageView, Layout,
    PipelineInput, PipelineStageFlags, RenderPassEncoder, Sampler, ShaderModuleInfo,
    VertexInputRate, VertexShader,
};

use super::{mat3_na_to_sierra, DrawNode, RendererContext};
use crate::{
    assets::material::Material,
    camera::Camera2,
    graphics::{
        vertex_layouts_for_pipeline, Graphics, SparseDescriptors, Transformation2, VertexLocation,
        VertexType,
    },
    rect::Rect,
    scene::Global2,
    sprite::Sprite,
    tiles::{TileMap, TileSet},
};

pub struct SpriteDraw {
    pipeline: DynamicGraphicsPipeline,
    pipeline_layout: <SpritePipeline as PipelineInput>::Layout,
    descriptors: SpriteDescriptors,
    set: SpriteDescriptorsInstance,
    textures: SparseDescriptors<ImageView>,
    sprites: Buffer,
    layer_range: Range<f32>,
}

#[shader_repr]
#[derive(Clone, Copy)]
struct Uniforms {
    camera: mat3,
}

impl Default for Uniforms {
    fn default() -> Self {
        Uniforms {
            camera: mat3::default(),
        }
    }
}

#[descriptors]
struct SpriteDescriptors {
    #[sampler]
    #[stages(Fragment)]
    sampler: Sampler,

    #[image(sampled)]
    #[stages(Fragment)]
    textures: [ImageView; 128],

    #[uniform]
    #[stages(Vertex)]
    uniforms: Uniforms,
}

#[pipeline]
struct SpritePipeline {
    #[set]
    set: SpriteDescriptors,
}

impl SpriteDraw {
    pub fn new(layer_range: Range<f32>, graphics: &mut Graphics) -> eyre::Result<Self> {
        assert!(
            layer_range.start >= 0.0 && layer_range.end > layer_range.start,
            "Layers range {}..{} is invalid",
            layer_range.start,
            layer_range.end,
        );

        let layer_start_bits = layer_range.start.to_bits();
        let layer_end_bits = layer_range.end.to_bits();

        const MAX_LAYERS_COUNT: u32 = u16::MAX as u32;

        assert!(
            layer_end_bits > MAX_LAYERS_COUNT
                && layer_start_bits < layer_end_bits - MAX_LAYERS_COUNT,
            "Layers range {}..{} is too small",
            layer_range.start,
            layer_range.end,
        );

        let vert_module = graphics.create_shader_module(ShaderModuleInfo::spirv(
            std::include_bytes!("sprite.vert.spv")
                .to_vec()
                .into_boxed_slice(),
        ))?;

        let frag_module = graphics.create_shader_module(ShaderModuleInfo::spirv(
            std::include_bytes!("sprite.frag.spv")
                .to_vec()
                .into_boxed_slice(),
        ))?;

        let pipeline_layout = SpritePipeline::layout(graphics)?;

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

        let dummy = graphics.create_image_view(sierra::ImageViewInfo::new(dummy))?;
        let textures = (0..128).map(|_| dummy.clone()).collect::<Vec<_>>();
        let textures = <[ImageView; 128]>::try_from(textures).unwrap();

        let sampler = graphics.create_sampler(sierra::SamplerInfo::linear())?;

        let sprites = graphics.create_buffer(sierra::BufferInfo {
            align: 255,
            size: std::mem::size_of::<SpriteInstance>() as u64 * 256,
            usage: sierra::BufferUsage::VERTEX | sierra::BufferUsage::TRANSFER_DST,
        })?;

        let set = pipeline_layout.set.instance();

        let (vertex_bindings, vertex_attributes) =
            vertex_layouts_for_pipeline(&[SpriteInstance::layout()]);

        Ok(SpriteDraw {
            pipeline: DynamicGraphicsPipeline::new(graphics_pipeline_desc! {
                vertex_bindings,
                vertex_attributes,
                vertex_shader: VertexShader::new(vert_module.clone(), "main"),
                fragment_shader: Some(FragmentShader::new(frag_module.clone(), "main")),
                layout: pipeline_layout.raw().clone(),
                depth_test: Some(DepthTest::LESS_WRITE),
            }),
            pipeline_layout,

            descriptors: SpriteDescriptors {
                sampler,
                textures,
                uniforms: Uniforms::default(),
            },
            set,
            textures: SparseDescriptors::new(),
            sprites,
            layer_range,
        })
    }
}

impl DrawNode for SpriteDraw {
    fn draw<'a, 'b: 'a>(
        &'b mut self,
        cx: RendererContext<'a, 'b>,

        encoder: &mut Encoder<'a>,
        render_pass: &mut RenderPassEncoder<'_, 'b>,
        camera: EntityId,
        _viewport: Extent2d,
    ) -> eyre::Result<()> {
        let (global, camera) = cx.world.query_one::<(&Global2, &Camera2)>(&camera)?;

        let view = global.iso.inverse().to_homogeneous();
        let affine = camera.affine().to_homogeneous();

        self.descriptors.uniforms.camera = mat3_na_to_sierra(affine * view);

        render_pass.bind_dynamic_graphics_pipeline(&mut self.pipeline, cx.graphics)?;

        let mut sprites = Vec::with_capacity_in(1024, &*cx.scope);

        for (_, (sprite, mat, global)) in cx.world.query_mut::<(&Sprite, &Material, &Global2)>() {
            let albedo = match &mat.albedo {
                Some(texture) => {
                    let (index, new) = self.textures.index(texture.image.clone());
                    if new {
                        self.descriptors.textures[index as usize] = texture.image.clone();
                    }
                    index
                }
                None => u32::MAX,
            };

            let layer_start_bits = self.layer_range.start.to_bits();
            let layer_bits = layer_start_bits + ((sprite.layer as u32) << 6);
            let layer = f32::from_bits(layer_bits);
            debug_assert!(layer < self.layer_range.end);

            let instance = SpriteInstance {
                pos: sprite.src.from_relative_to(&sprite.world),
                uv: sprite.tex,
                layer,
                albedo,
                albedo_factor: {
                    let [r, g, b, a] = mat.albedo_factor;
                    LinSrgba::new(r, g, b, a)
                },
                transform: Transformation2(global.iso.to_homogeneous().into()),
            };

            sprites.push(instance);
        }

        for (_, (map, set, global)) in cx.world.query_mut::<(&TileMap, &TileSet, &Global2)>() {
            let hc = map.cell_size * 0.5;

            for (j, row) in map.cells.chunks(map.width).enumerate() {
                for (i, &cell) in row.iter().enumerate() {
                    let tile = match set.tiles.get(cell) {
                        None => {
                            return Err(eyre::eyre!("Missing tile '{}' in the tileset", cell));
                        }
                        Some(tile) => tile,
                    };

                    let albedo = match &tile.texture {
                        Some(texture) => {
                            let (index, new) = self.textures.index(texture.image.clone());
                            if new {
                                self.descriptors.textures[index as usize] = texture.image.clone();
                            }
                            index
                        }
                        None => u32::MAX,
                    };

                    let instance = SpriteInstance {
                        pos: Rect {
                            left: i as f32 * map.cell_size - hc,
                            right: i as f32 * map.cell_size + hc,
                            top: j as f32 * map.cell_size - hc,
                            bottom: j as f32 * map.cell_size + hc,
                        },
                        uv: tile.uv,
                        layer: self.layer_range.end,
                        albedo,
                        albedo_factor: LinSrgba::new(1.0, 1.0, 1.0, 1.0),
                        transform: Transformation2(global.iso.to_homogeneous().into()),
                    };

                    sprites.push(instance);
                }
            }
        }

        tracing::debug!("Rendering {} sprites", sprites.len());

        let updated = self.set.update(&self.descriptors, cx.graphics, encoder)?;

        render_pass.bind_graphics_descriptors(&self.pipeline_layout, updated);

        let sprite_count = sprites.len() as u32;

        if self.sprites.info().size < sprite_count as u64 * size_of::<SpriteInstance>() as u64 {
            self.sprites = cx.graphics.create_buffer(sierra::BufferInfo {
                align: 255,
                size: std::mem::size_of::<SpriteInstance>() as u64
                    * (sprite_count as u64).next_power_of_two(),
                usage: sierra::BufferUsage::VERTEX | sierra::BufferUsage::TRANSFER_DST,
            })?;
        }

        cx.graphics
            .upload_buffer_with(&self.sprites, 0, sprites.leak(), encoder)?;

        encoder.memory_barrier(
            PipelineStageFlags::TRANSFER,
            AccessFlags::TRANSFER_WRITE,
            PipelineStageFlags::VERTEX_INPUT,
            AccessFlags::VERTEX_ATTRIBUTE_READ,
        );

        render_pass.bind_vertex_buffers(0, &[(&self.sprites, 0)]);
        render_pass.draw(0..6, 0..sprite_count);

        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SpriteInstance {
    pos: Rect,
    uv: Rect,
    layer: f32,
    albedo: u32,
    albedo_factor: LinSrgba<f32>,
    transform: Transformation2,
}

unsafe impl bytemuck::Zeroable for SpriteInstance {}
unsafe impl bytemuck::Pod for SpriteInstance {}

impl VertexType for SpriteInstance {
    const LOCATIONS: &'static [VertexLocation] = {
        let mut offset = 0;
        &[
            vertex_location!(offset, Rect),
            vertex_location!(offset, Rect),
            vertex_location!(offset, f32 as "Layer"),
            vertex_location!(offset, u32 as "Albedo"),
            vertex_location!(offset, LinSrgba<f32>),
            vertex_location!(offset, [f32; 3] as "Transform2.0"),
            vertex_location!(offset, [f32; 3] as "Transform2.1"),
            vertex_location!(offset, [f32; 3] as "Transform2.2"),
        ]
    };
    const RATE: VertexInputRate = VertexInputRate::Instance;
}
