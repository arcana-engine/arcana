use std::sync::Arc;

use egui::{ClippedMesh, CtxRef, FontImage};
use sierra::ImageView;
use winit::{event::WindowEvent, window::Window};

pub struct EguiResource {
    ctx: CtxRef,
    state: egui_winit::State,
    meshes: Vec<ClippedMesh>,
    textures: Vec<ImageView>,
}

impl EguiResource {
    pub fn new(window: &Window) -> Self {
        EguiResource {
            ctx: CtxRef::default(),
            state: egui_winit::State::new(window),
            meshes: Vec::new(),
            textures: Vec::new(),
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) -> bool {
        self.state.on_event(&self.ctx, event)
    }

    pub fn scale_factor(&self) -> f32 {
        self.state.pixels_per_point()
    }

    pub fn font_image(&self) -> Arc<FontImage> {
        self.ctx.font_image()
    }

    pub fn meshes(&self) -> &[ClippedMesh] {
        &self.meshes
    }

    pub fn add_texture(&mut self, view: ImageView) -> usize {
        let idx = self.textures.len();
        self.textures.push(view);
        idx
    }

    pub fn run(&mut self, window: &Window, run_ui: impl FnOnce(&CtxRef)) {
        let input = self.state.take_egui_input(window);
        self.ctx.begin_frame(input);
        run_ui(&self.ctx);
        let (output, shapes) = self.ctx.end_frame();
        self.state.handle_output(window, &self.ctx, output);
        self.meshes = self.ctx.tessellate(shapes);
    }
}
