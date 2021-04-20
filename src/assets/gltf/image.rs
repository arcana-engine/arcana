use {
    super::{
        super::image::image_view_from_dyn_image, GltfBuildContext, GltfDecoded, GltfLoadingError,
    },
    crate::graphics::Graphics,
    sierra::{CreateImageError, ImageView},
    std::collections::hash_map::Entry,
};

impl GltfBuildContext<'_> {
    pub fn get_image(&mut self, image: gltf::Image) -> Result<ImageView, GltfLoadingError> {
        match self.images.entry(image.index()) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => Ok(entry
                .insert(create_image(self.decoded, image, &mut self.graphics)?)
                .clone()),
        }
    }
}

fn create_image(
    repr: &GltfDecoded,
    image: gltf::Image,
    graphics: &mut Graphics,
) -> Result<ImageView, GltfLoadingError> {
    let bytes = match image.source() {
        gltf::image::Source::View { view, .. } => {
            let bytes = match view.buffer().source() {
                gltf::buffer::Source::Bin => repr.gltf.blob.as_deref(),
                gltf::buffer::Source::Uri(uri) => repr.sources.get(uri).map(|b| &**b),
            };
            let bytes = bytes.ok_or(GltfLoadingError::MissingSource)?;
            if bytes.len() < view.offset() + view.length() {
                return Err(GltfLoadingError::ViewOutOfBound);
            }

            &bytes[view.offset()..][..view.length()]
        }
        gltf::image::Source::Uri { uri, .. } => repr
            .sources
            .get(uri)
            .map(|b| &**b)
            .ok_or(GltfLoadingError::MissingSource)?,
    };

    let dyn_image = image::load_from_memory(bytes)?;
    let image = dyn_image.to_rgba8();

    match image_view_from_dyn_image(&image::DynamicImage::ImageRgba8(image), graphics) {
        Ok(view) => Ok(view),
        Err(CreateImageError::OutOfMemory { source }) => {
            Err(GltfLoadingError::OutOfMemory { source })
        }
        Err(CreateImageError::Unsupported { info }) => {
            Err(GltfLoadingError::UnsupportedImage { info })
        }
    }
}
