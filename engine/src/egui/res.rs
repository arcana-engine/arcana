use egui::{ClippedPrimitive, ColorImage, Context, FontImage, ImageData, TextureId, TexturesDelta};
use hashbrown::hash_map::{Entry, HashMap};
use sierra::{
    Access, Encoder, Extent2, Extent3, ImageInfo, ImageView, ImageViewInfo, Offset3, OutOfMemory,
    SubresourceLayers,
};
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget, window::Window};

use crate::graphics::{Graphics, UploadImage};

pub struct EguiResource {
    ctx: Context,
    state: egui_winit::State,
    primitives: Vec<ClippedPrimitive>,
    egui_textures: HashMap<u64, ImageView>,
    user_textures: HashMap<u64, ImageView>,
    textures_delta: Option<TexturesDelta>,

    free_textures: Vec<TextureId>,
}

impl EguiResource {
    pub fn new<T>(events: &EventLoopWindowTarget<T>) -> Self {
        EguiResource {
            ctx: Context::default(),
            state: egui_winit::State::new(events),
            primitives: Vec::new(),
            egui_textures: HashMap::new(),
            user_textures: HashMap::new(),
            textures_delta: None,
            free_textures: Vec::new(),
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) -> bool {
        self.state.on_event(&self.ctx, event)
    }

    pub fn scale_factor(&self) -> f32 {
        self.state.pixels_per_point()
    }

    pub fn primitives(&self) -> &[ClippedPrimitive] {
        &self.primitives
    }

    pub fn add_texture(&mut self, id: u64, view: ImageView) {
        self.user_textures.insert(id, view);
    }

    pub fn run(&mut self, window: &Window, run_ui: impl FnOnce(&Context)) {
        let input = self.state.take_egui_input(window);
        self.ctx.begin_frame(input);
        run_ui(&self.ctx);
        let output = self.ctx.end_frame();
        self.state
            .handle_platform_output(window, &self.ctx, output.platform_output);
        self.primitives = self.ctx.tessellate(output.shapes);
        self.textures_delta = Some(output.textures_delta);
    }

    pub fn update_egui_textures(
        &mut self,
        encoder: &mut Encoder,
        graphics: &mut Graphics,
    ) -> Result<(), OutOfMemory> {
        if let Some(textures_delta) = self.textures_delta.take() {
            for (id, delta) in &textures_delta.set {
                let id = match *id {
                    TextureId::Managed(id) => id,
                    TextureId::User(id) => {
                        tracing::error!("Egui provides delta for user-texture '{}'", id);
                        continue;
                    }
                };

                let pos = delta.pos.unwrap_or([0; 2]);

                let (size, pixels, format) = match &delta.image {
                    ImageData::Color(ColorImage { size, pixels }) => (
                        *size,
                        bytemuck::cast_slice::<_, u8>(&pixels[..]),
                        sierra::Format::RGBA8Srgb,
                    ),
                    ImageData::Font(FontImage { size, pixels }) => (
                        *size,
                        bytemuck::cast_slice(&pixels[..]),
                        sierra::Format::R8Srgb,
                    ),
                };

                let (view, new) = match self.egui_textures.entry(id) {
                    Entry::Vacant(entry) => {
                        let image = graphics.create_image(ImageInfo {
                            extent: Extent2::new(size[0] as _, size[1] as _).into(),
                            format,
                            levels: 1,
                            layers: 1,
                            samples: sierra::Samples1,
                            usage: sierra::ImageUsage::TRANSFER_DST | sierra::ImageUsage::SAMPLED,
                        })?;

                        let view = graphics.create_image_view(ImageViewInfo {
                            mapping: sierra::swizzle!(111r),
                            ..ImageViewInfo::new(image)
                        })?;
                        (&*entry.insert(view), true)
                    }
                    Entry::Occupied(entry) => (&*entry.into_mut(), false),
                };

                let image = &view.info().image;

                graphics.upload_image_with(
                    UploadImage {
                        image,
                        offset: Offset3::new(pos[0] as _, pos[1] as _, 0),
                        extent: Extent3::new(size[0] as _, size[1] as _, 1),
                        layers: SubresourceLayers::color(0, 0..1),
                        old_layout: if new {
                            None
                        } else {
                            Some(sierra::Layout::ShaderReadOnlyOptimal)
                        },
                        new_layout: sierra::Layout::ShaderReadOnlyOptimal,
                        old_access: Access::SHADER_SAMPLED_READ,
                        new_access: Access::SHADER_SAMPLED_READ,
                        format,
                        row_length: 0,
                        image_height: 0,
                    },
                    &pixels[..],
                    encoder,
                )?;
            }

            for id in textures_delta.free {
                if let TextureId::User(id) = id {
                    tracing::error!("Egui attempts to free for user-texture '{}'", id);
                    continue;
                }
                self.free_textures.push(id);
            }
        }

        Ok(())
    }

    pub fn free_textures(&mut self) -> impl Iterator<Item = TextureId> + '_ {
        struct FreeTextures<'a> {
            iter: std::vec::Drain<'a, TextureId>,
            egui_textures: &'a mut HashMap<u64, ImageView>,
            user_textures: &'a mut HashMap<u64, ImageView>,
        }

        impl Iterator for FreeTextures<'_> {
            type Item = TextureId;

            fn next(&mut self) -> Option<TextureId> {
                let id = self.iter.next()?;
                match id {
                    TextureId::Managed(id) => {
                        self.egui_textures.remove(&id);
                    }
                    TextureId::User(id) => {
                        self.user_textures.remove(&id);
                    }
                }
                Some(id)
            }
        }

        impl Drop for FreeTextures<'_> {
            fn drop(&mut self) {
                for _ in self {}
            }
        }

        FreeTextures {
            iter: self.free_textures.drain(..),
            egui_textures: &mut self.egui_textures,
            user_textures: &mut self.user_textures,
        }
    }

    pub fn get_texture(&self, id: TextureId) -> Option<&ImageView> {
        match id {
            TextureId::Managed(id) => self.egui_textures.get(&id),
            TextureId::User(id) => self.user_textures.get(&id),
        }
    }
}
