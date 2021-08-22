use super::{mat3_na_to_sierra, DrawNode, RendererContext};
use crate::{
    camera::Camera2,
    graphics::{
        material::Material,
        sprite::{Rect, Sprite},
        vertex::{vertex_layouts_for_pipeline, Semantics, VertexLocation, VertexType},
        Graphics, SparseDescriptors,
    },
    scene::Global2,
};

use hecs::Entity;
use sierra::{
    descriptors, graphics_pipeline_desc, mat3, pipeline, shader_repr, Buffer, DepthTest,
    DynamicGraphicsPipeline, Encoder, Format, FragmentShader, ImageView, Layout, PipelineInput,
    RenderPassEncoder, Sampler, ShaderModuleInfo, VertexInputRate, VertexShader,
};
use std::{borrow::Cow, convert::TryFrom};

pub struct SpriteDraw {
    pipeline: DynamicGraphicsPipeline,
    pipeline_layout: <SpritePipeline as PipelineInput>::Layout,
    descriptors: SpriteDescriptors,
    set: SpriteDescriptorsInstance,
    textures: SparseDescriptors<ImageView>,
    sprites: Buffer,
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

    #[sampled_image]
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
    pub fn new(graphics: &mut Graphics) -> eyre::Result<Self> {
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
            4,
            1,
            &[255u8, 255, 255, 255],
        )?;

        let dummy = graphics.create_image_view(sierra::ImageViewInfo::new(dummy))?;
        let textures = (0..128).map(|_| dummy.clone()).collect::<Vec<_>>();
        let textures = <[ImageView; 128] as TryFrom<Vec<_>>>::try_from(textures).unwrap();

        let sampler = graphics.create_sampler(sierra::SamplerInfo::linear())?;

        let sprites = graphics.create_buffer(sierra::BufferInfo {
            align: 255,
            size: std::mem::size_of::<SpriteInstance>() as u64 * 65536,
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
        })
    }
}

impl DrawNode for SpriteDraw {
    fn draw<'a>(
        &'a mut self,
        cx: RendererContext<'a>,
        fence_index: usize,
        encoder: &mut Encoder<'a>,
        mut render_pass: RenderPassEncoder<'_, 'a>,
        camera: Entity,
    ) -> eyre::Result<()> {
        let view = cx
            .world
            .get_mut::<Global2>(camera)?
            .iso
            .inverse()
            .to_homogeneous();

        let affine = cx
            .world
            .get_mut::<Camera2>(camera)?
            .affine()
            .to_homogeneous();

        self.descriptors.uniforms.camera = mat3_na_to_sierra(affine * view);

        render_pass.bind_dynamic_graphics_pipeline(&mut self.pipeline, cx.graphics)?;

        let mut sprites = Vec::new_in(&*cx.scope);
        let mut writes = Vec::new_in(&*cx.scope);
        for (_, (sprite, mat, global)) in cx.world.query_mut::<(&Sprite, &Material, &Global2)>() {
            let albedo = match &mat.albedo_coverage {
                Some(texture) => {
                    let (index, new) = self.textures.index(texture.image.clone());
                    if new {
                        self.descriptors.textures[index as usize] = texture.image.clone();
                    }
                    index
                }
                None => !0,
            };

            let instance = SpriteInstance {
                pos: sprite.src.from_relative_to(&sprite.world),
                uv: sprite.tex,
                layer: sprite.layer,
                albedo,
                albedo_factor: {
                    let [r, g, b] = mat.albedo_factor;
                    [r.into(), g.into(), b.into()]
                },
                transform: global.iso.to_homogeneous().into(),
            };

            sprites.push(instance);
        }

        let updated = self.set.update(
            &self.descriptors,
            fence_index,
            cx.graphics,
            &mut writes,
            encoder,
        )?;

        cx.graphics.update_descriptor_sets(&writes, &[]);
        render_pass.bind_graphics_descriptors(&self.pipeline_layout, updated);

        let sprite_count = sprites.len() as u32;

        cx.graphics
            .upload_buffer_with(&self.sprites, 0, sprites.leak(), encoder)?;

        let buffers = render_pass.scope().to_scope([(&self.sprites, 0)]);
        render_pass.bind_vertex_buffers(0, buffers);
        render_pass.draw(0..6, 0..sprite_count);

        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct SpriteInstance {
    pos: Rect,
    uv: Rect,
    layer: u32,
    albedo: u32,
    albedo_factor: [f32; 3],
    transform: [[f32; 3]; 3],
}

unsafe impl bytemuck::Zeroable for SpriteInstance {}
unsafe impl bytemuck::Pod for SpriteInstance {}

impl VertexType for SpriteInstance {
    const NAME: &'static str = "SpriteInstance";
    const LOCATIONS: &'static [VertexLocation] = &[
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: 0,
            semantics: Semantics::Custom(Cow::Borrowed("pos_aabb")),
        },
        VertexLocation {
            format: Format::RGBA32Sfloat,
            offset: 16,
            semantics: Semantics::Custom(Cow::Borrowed("uv_aabb")),
        },
        VertexLocation {
            format: Format::R32Uint,
            offset: 32,
            semantics: Semantics::Custom(Cow::Borrowed("sprite_layer")),
        },
        VertexLocation {
            format: Format::R32Uint,
            offset: 36,
            semantics: Semantics::Custom(Cow::Borrowed("albedo")),
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 40,
            semantics: Semantics::Custom(Cow::Borrowed("albedo_factor")),
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 52,
            semantics: Semantics::Custom(Cow::Borrowed("transform2_0")),
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 64,
            semantics: Semantics::Custom(Cow::Borrowed("transform2_1")),
        },
        VertexLocation {
            format: Format::RGB32Sfloat,
            offset: 76,
            semantics: Semantics::Custom(Cow::Borrowed("transform2_2")),
        },
    ];
    const RATE: VertexInputRate = VertexInputRate::Instance;
}
