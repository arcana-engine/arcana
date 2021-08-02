mod aseprite;
mod gltf;
mod image;
mod material;
mod sampler;
mod sprite_sheet;

#[cfg(target_os = "wasi")]
pub use goods_treasury_import::ffi::{
    treasury_importer_alloc, treasury_importer_dealloc, treasury_importer_import_trampoline,
    treasury_importer_name_source_native_trampoline,
};

goods_treasury_import::generate_imports_and_exports! {
    &aseprite::SpriteSheetImporter,
    &image::ImageImporter,
    &sprite_sheet::SpriteSheetEnrich,
    &gltf::GltfObjectImporter,
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}
