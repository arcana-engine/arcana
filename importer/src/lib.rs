mod image;
use {
    std::sync::Arc,
    treasury_import::{Importer, MAGIC},
};

/// This is required to minimize chances that random shared library
/// would export symbols with same name and cause UB.
/// If magic number does not match shared library won't be used.
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static treasury_import_magic_number: u32 = MAGIC;

/// Import version to check that both rustc version and `goods-import` dependency version
/// match. Otherwise using `get_treasury_importers` may cause UB.
#[no_mangle]
pub fn get_treasury_import_version() -> &'static str {
    treasury_import::treasury_import_version()
}

/// Returns array of importers from this library.
#[no_mangle]
pub fn get_treasury_importers() -> Vec<Arc<dyn Importer>> {
    vec![Arc::new(crate::image::ImageImporter)]
}
