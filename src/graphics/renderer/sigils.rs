use std::{collections::HashMap, convert::TryFrom, mem::size_of_val};

use goods::AssetId;
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
    assets::FontFaces,
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
    images: HashMap<AssetId, ImageView>,
    fonts: HashMap<AssetId, FontFaces>,
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

        // Generate meshes and glyphs in UI
        let (layers, meshes, glyphs) = ui.render(&[], cx.scope);

        for layer in layers {
            let meshes = &meshes[layer.meshes_start..][..layer.meshes_count];
            let glyphs = &glyphs[layer.glyphs_start..][..layer.glyphs_count];
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
