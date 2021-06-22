mod raster;

use {
    arcana::*,
    std::collections::hash_map::{Entry, HashMap},
};

struct RaysRenderer {
    blases: HashMap<graphics::Mesh, graphics::AccelerationStructure>,
}

impl graphics::Renderer for RaysRenderer {
    fn new(graphics: &mut graphics::Graphics) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        Ok(RaysRenderer {
            blases: HashMap::new(),
        })
    }

    fn render(
        &mut self,
        cx: graphics::RendererContext<'_>,
        viewports: &mut [&mut Viewport],
    ) -> eyre::Result<()> {
        let mut encoder = cx.graphics.create_encoder(cx.bump)?;

        let mut insert_blasses = bumpalo::collections::Vec::new_in(cx.bump);

        for (e, mesh) in cx
            .world
            .query_mut::<&graphics::Mesh>()
            .with::<graphics::Material>()
            .without::<graphics::AccelerationStructure>()
        {
            let blas = match self.blases.entry(mesh.clone()) {
                Entry::Occupied(entry) => entry.get().clone(),
                Entry::Vacant(entry) => {
                    let blas = mesh.build_triangles_blas(&mut encoder, cx.graphics, cx.bump)?;
                    entry.insert(blas).clone()
                }
            };
            insert_blasses.push((e, blas));
        }

        for (e, blas) in insert_blasses {
            cx.world.insert_one(e, blas).unwrap();
        }

        Ok(())
    }
}

fn main() {
    game3(|mut game| async move {
        game.scheduler.add_system(camera::FreeCameraSystem);
        game.control
            .assume_control(
                game.viewport.camera(),
                camera::FreeCamera3Controller::new(),
                &mut game.world,
            )
            .unwrap();
        Ok(game)
    })
}
