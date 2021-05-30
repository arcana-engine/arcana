mod aseprite;
mod image;
mod sprite_sheet;

#[cfg(target_os = "wasi")]
pub use treasury_import::ffi::{
    treasury_importer_alloc, treasury_importer_dealloc, treasury_importer_import_trampoline,
    treasury_importer_name_source_native_trampoline,
};

treasury_import::generate_imports_and_exports! {
    &aseprite::SpriteSheetImporter,
    &image::ImageImporter,
    &sprite_sheet::SpriteSheetEnrich,
}
