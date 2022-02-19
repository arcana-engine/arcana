use arcana::{
    egui::{self, EguiDraw, EguiFunnel, EguiResource},
    game::{game2, MainWindow},
    graphics::{simple::SimpleRenderer, DrawNode},
    system::SystemContext,
};
use blueprint::Blueprint;

fn main() {
    game2(|mut game| async move {
        let renderer =
            SimpleRenderer::with_multiple(vec![
                Box::new(EguiDraw::new(&mut game.graphics)?) as Box<dyn DrawNode>
            ]);
        game.renderer = Some(Box::new(renderer));

        let window = game
            .res
            .get::<MainWindow>()
            .expect("Window must be created");
        let egui = EguiResource::new(window);
        game.res.insert(egui);

        #[derive(Erased, Clone, Default)]
        struct Foo {
            small: i8,
            large: u32,
        }

        #[derive(Erased, Clone, Default)]
        struct Bar {
            foo: Foo,
            real: f32,
            list: Vec<f32>,
        }

        let mut bar = Bar::default();

        // Add GUI system
        game.scheduler.add_system(move |cx: SystemContext<'_>| {
            let (egui, window) = cx.res.query::<(&mut EguiResource, &MainWindow)>();

            egui.run(window, |ctx| {
                egui::Window::new("Edit").show(ctx, |ui| {
                    let res = egui_blueprint::edit_value(ui, &mut bar);
                });
            });
        });

        game.funnel = Some(Box::new(EguiFunnel));

        Ok(game)
    })
}
