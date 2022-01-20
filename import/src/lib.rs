// mod aseprite;
// mod gltf;
mod image;
// mod material;
// mod sampler;
// mod sprite_sheet;

treasury_import::make_treasury_importers_library! {
    [png, jpg, jpeg] qoiconv : image -> qoi = &image::ImageImporter;
}
