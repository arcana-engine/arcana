mod aseprite;
// mod gltf;
mod copy;
mod image;
// mod material;
// mod sampler;
mod sprite_sheet;
mod tiles;

treasury_import::make_treasury_importers_library! {
    [png, jpg, jpeg] qoiconv : image -> qoi = &image::ImageImporter;
    [json] aseprite.spritesheet : aseprite.spritesheet -> arcana.spritesheet = &aseprite::SpriteSheetImporter;
    [json] arcana.tileset : arcana.tileset -> arcana.tileset = &tiles::TileSetImporter;
    [json] arcana.tilemap : arcana.tilemap -> arcana.tilemap = &tiles::TileMapImporter;
}

// fn is_default<T>(v: &T) -> bool
// where
//     T: Default + PartialEq,
// {
//     *v == T::default()
// }
