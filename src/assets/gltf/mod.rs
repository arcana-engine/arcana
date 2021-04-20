mod image;
mod material;
mod prefab;
mod primitive;
mod sampler;
mod skin;

use {
    super::{asset::Asset, format::Format, Error, Loader},
    crate::graphics::{Graphics, Material, Mesh},
    ::image::ImageError,
    futures::future::{try_join_all, BoxFuture},
    gltf::accessor::{DataType, Dimensions},
    sierra::{Buffer, BufferUsage, ImageInfo, ImageView, OutOfMemory, Sampler},
    std::{collections::HashMap, sync::Arc},
    url::Url,
};

pub use prefab::Gltf;

#[derive(Clone, Debug)]
pub struct GltfRenderable {
    mesh: Mesh,
    material: Material,
    transform: na::Matrix4<f32>,
}

#[derive(Clone, Copy, Debug)]
pub struct GltfFormat {
    pub mesh_vertices_usage: BufferUsage,
    pub mesh_indices_usage: BufferUsage,
}

impl Default for GltfFormat {
    fn default() -> Self {
        Self::for_raster()
    }
}

impl GltfFormat {
    pub fn for_raster() -> Self {
        GltfFormat {
            mesh_indices_usage: BufferUsage::INDEX,
            mesh_vertices_usage: BufferUsage::VERTEX,
        }
    }

    pub fn for_raytracing() -> Self {
        GltfFormat {
            mesh_indices_usage: BufferUsage::STORAGE | BufferUsage::DEVICE_ADDRESS,
            mesh_vertices_usage: BufferUsage::STORAGE | BufferUsage::DEVICE_ADDRESS,
        }
    }
}

/// gltf scenes with initialized resources.
#[derive(Clone, Debug)]
pub struct GltfAsset {
    gltf: gltf::Gltf,
    renderables: Arc<[Box<[GltfRenderable]>]>,
}

struct GltfBuildContext<'a> {
    decoded: &'a GltfDecoded,
    graphics: &'a mut Graphics,
    // buffers: HashMap<usize, Buffer>,
    images: HashMap<usize, ImageView>,
    samplers: HashMap<Option<usize>, Sampler>,
    materials: HashMap<Option<usize>, Material>,
    primitives: HashMap<usize, GltfRenderable>,
    // default_sampler: Option<Sampler>,
}

impl Asset for GltfAsset {
    type Error = GltfLoadingError;
    type Builder = Graphics;
    type Decoded = GltfDecoded;

    fn build(decoded: GltfDecoded, graphics: &mut Graphics) -> Result<Self, GltfLoadingError> {
        // let images = repr
        //     .gltf
        //     .images()
        //     .map(|image| load_gltf_image(&repr, image, ctx))
        //     .collect::<Result<Vec<_>, _>>()?;

        // let samplers = repr
        //     .gltf
        //     .samplers()
        //     .map(|sampler| load_gltf_sampler(sampler, ctx))
        //     .collect::<Result<Vec<_>, _>>()?;

        // let mut default_sampler = None;

        // let textures = repr
        //     .gltf
        //     .textures()
        //     .map(|texture| {
        //         load_gltf_texture(
        //             texture,
        //             &images,
        //             &samplers,
        //             &mut default_sampler,
        //             ctx,
        //         )
        //     })
        //     .collect::<Result<Vec<_>, _>>()?;

        let mut ctx = GltfBuildContext {
            decoded: &decoded,
            graphics,
            // buffers: HashMap::new(),
            images: HashMap::new(),
            samplers: HashMap::new(),
            materials: HashMap::new(),
            primitives: HashMap::new(),
            // default_sampler: None,
        };

        // let materials = repr
        //     .gltf
        //     .materials()
        //     .map(|material| load_gltf_material(material, &mut ctx))
        //     .collect::<Result<Vec<_>, _>>()?;

        let renderables = decoded
            .gltf
            .meshes()
            .map(|mesh| {
                mesh.primitives()
                    .map(|prim| ctx.get_primitive(prim))
                    .collect::<Result<_, _>>()
            })
            .collect::<Result<_, _>>()?;

        Ok(GltfAsset {
            gltf: decoded.gltf,
            renderables,
        })
    }
}

/// Intermediate gltf representation.
/// Contains parsed gltf tree and all sources loaded.
pub struct GltfDecoded {
    gltf: gltf::Gltf,
    sources: HashMap<String, Box<[u8]>>,
    config: GltfFormat,
}

#[derive(Debug, thiserror::Error)]
pub enum GltfFormatError {
    #[error(transparent)]
    Gltf(#[from] gltf::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Source(Error),
}

impl Format<GltfAsset> for GltfFormat {
    type Error = GltfFormatError;
    type Fut = BoxFuture<'static, Result<GltfDecoded, Self::Error>>;

    fn decode(
        self,
        bytes: Box<[u8]>,
        key: &str,
        loader: Loader,
    ) -> BoxFuture<'static, Result<GltfDecoded, GltfFormatError>> {
        match gltf::Gltf::from_slice(&bytes) {
            Err(err) => Box::pin(async move { Err(err.into()) }),
            Ok(gltf) => {
                let mut sources = Vec::new();

                enum UrlOrString {
                    Url(Url),
                    String(String),
                }

                impl AsRef<str> for UrlOrString {
                    fn as_ref(&self) -> &str {
                        match self {
                            UrlOrString::Url(url) => url.as_ref(),
                            UrlOrString::String(string) => string.as_ref(),
                        }
                    }
                }

                let combine_uri = |url: &str| match url.parse::<Url>() {
                    Ok(url) => UrlOrString::Url(url),
                    Err(_) => {
                        let mut path = match key.rfind('/') {
                            Some(pos) => key[..pos + 1].to_owned(),
                            None => String::new(),
                        };
                        path.push_str(url);
                        UrlOrString::String(path)
                    }
                };

                for buffer in gltf.buffers() {
                    match buffer.source() {
                        gltf::buffer::Source::Bin => {}
                        gltf::buffer::Source::Uri(uri) => {
                            sources.push(loader.read(combine_uri(uri).as_ref()))
                        }
                    }
                }

                for image in gltf.images() {
                    match image.source() {
                        gltf::image::Source::View { .. } => {}
                        gltf::image::Source::Uri { uri, .. } => {
                            sources.push(loader.read(combine_uri(uri).as_ref()))
                        }
                    }
                }

                let sources = try_join_all(sources);

                Box::pin(async move {
                    let sources = sources.await.map_err(GltfFormatError::Source)?;

                    let buffers_uri = gltf.buffers().filter_map(|b| match b.source() {
                        gltf::buffer::Source::Bin => None,
                        gltf::buffer::Source::Uri(uri) => Some(uri.to_owned()),
                    });

                    let images_uri = gltf.images().filter_map(|b| match b.source() {
                        gltf::image::Source::View { .. } => None,
                        gltf::image::Source::Uri { uri, .. } => Some(uri.to_owned()),
                    });

                    Ok(GltfDecoded {
                        sources: buffers_uri.chain(images_uri).zip(sources).collect(),
                        config: self,
                        gltf,
                    })
                })
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GltfLoadingError {
    #[error(transparent)]
    GltfError {
        #[from]
        source: gltf::Error,
    },

    #[error("GLTF with no scenes")]
    NoScenes,

    #[error("Failed to allocate GPU resource")]
    OutOfMemory {
        #[from]
        source: OutOfMemory,
    },

    #[error("Accessor has unexpected dimensions `{unexpected:?}`. Expected `{expected:?}`")]
    UnexpectedDimensions {
        unexpected: Dimensions,
        expected: &'static [Dimensions],
    },

    #[error("Accessor has unexpected data type `{unexpected:?}`. Expected `{expected:?}`")]
    UnexpectedDataType {
        unexpected: DataType,
        expected: &'static [DataType],
    },

    #[error("Sparse accessors are not supported")]
    SparseAccessorUnsupported,

    #[error("Accessor does not fit the view")]
    AccessorOutOfBound,

    #[error("View does not fit the source")]
    ViewOutOfBound,

    #[error("Source does not exist")]
    MissingSource,

    #[error("Unsupported mesh without position attribute")]
    MissingPositionAttribute,

    #[error("Texture referenced in material not found in textures array")]
    MissingTexture,

    #[error("Unsupported mesh topology")]
    UnsupportedTopology { unsupported: gltf::mesh::Mode },

    #[error("Failed to load image data: `{source}`")]
    ImageError {
        #[from]
        source: ImageError,
    },

    #[error("Combination paramters `{info:?}` is unsupported")]
    UnsupportedImage { info: ImageInfo },
}

fn align_vec(bytes: &mut Vec<u8>, align_mask: usize) {
    let new_size = (bytes.len() + align_mask) & !align_mask;
    bytes.resize(new_size, 0xfe);
}
