use msdfgen_lib as _; // forces linking with msdfgen library

use notosans::REGULAR_TTF as FONT;
use std::fs::File;
use ttf_parser::Font;

use msdfgen::{Bitmap, FontExt, Range, EDGE_THRESHOLD, OVERLAP_SUPPORT};

fn main() {
    let font = Font::from_data(&FONT, 0).unwrap();
    let glyph = font.glyph_index('A').unwrap();
    let mut shape = font.glyph_shape(glyph).unwrap();

    if !shape.validate() {
        panic!("Invalid shape");
    }
    shape.normalize();

    let bounds = shape.get_bounds();

    let width = 32;
    let height = 32;

    let mut bitmap = Bitmap::new(width, height);

    println!("bounds: {:?}", bounds);

    shape.edge_coloring_simple(3.0, 0);

    let framing = bounds
        .autoframe(width, height, Range::Px(4.0), None)
        .unwrap();

    println!("framing: {:?}", framing);

    shape.generate_msdf(&mut bitmap, &framing, EDGE_THRESHOLD, OVERLAP_SUPPORT);

    let mut output = File::create("A-msdf.png").unwrap();
    bitmap.write_png(&mut output).unwrap();
}

impl Importer for FontImporter {
    fn name(&self) -> &str {
        "arcana.font"
    }

    fn source(&self) -> &str {
        "ttf"
    }

    fn native(&self) -> &str {
        "arcana.font"
    }

    fn import(
        &self,
        source_path: &Path,
        native_path: &Path,
        _registry: &mut dyn Registry,
    ) -> eyre::Result<()> {

        

    }
}
