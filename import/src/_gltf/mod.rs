mod animation;
mod collider;
mod image;
mod material;
mod mesh;
mod renderable;
mod sampler;
mod scene;
mod skin;

use {
    gltf::accessor::{DataType, Dimensions},
    std::{
        collections::HashMap,
        fmt::{self, Debug},
        sync::Arc,
    },
    uuid::Uuid,
};

#[derive(Clone, Debug)]
pub struct Renderable {
    pub mesh: Uuid,
    pub material: Uuid,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub renderables: Option<Arc<[Renderable]>>,
    pub colliders: Option<Arc<[Collider]>>,
    pub skin: Option<Skin>,
}

#[derive(Clone, Debug)]
pub struct Skin {
    inverse_binding_matrices: Option<Arc<[na::Matrix4<f32>]>>,
    joints: Arc<[usize]>,
}

#[derive(Clone, Debug)]
pub enum SamplerOutput {
    Scalar(Arc<[f32]>),
    Vec2(Arc<[[f32; 2]]>),
    Vec3(Arc<[[f32; 3]]>),
    Vec4(Arc<[[f32; 4]]>),
}

#[derive(Clone, Debug)]
pub struct Channel {
    node: usize,
    property: gltf::animation::Property,
    input: Arc<[f32]>,
    output: SamplerOutput,
    interpolation: gltf::animation::Interpolation,
}

#[derive(Clone, Debug)]
pub struct Animation {
    channels: Arc<[Channel]>,
}

#[derive(Clone, Copy)]
pub enum ColliderKind {
    AABB,
    Convex,
    TriMesh,
}

#[derive(Clone)]
pub enum Collider {
    AABB { extent: na::Vector3<f32> },
    Convex { poits: Vec<na::Vector3<f32>> },
    TriMesh { uuid: Uuid },
}

impl Debug for Collider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GltfCollider")
            .field("shape", &self.shape.shape_type())
            .finish()
    }
}

/// GLTF scenes with initialized resources.
#[derive(Clone, Debug)]
pub struct GltfAsset {
    pub gltf: gltf::Gltf,
    pub meshes: Arc<[Mesh]>,
    pub animations: Arc<[Animation]>,
}

struct GltfBuildContext<'a> {
    decoded: &'a GltfDecoded,
    graphics: &'a mut Graphics,
    images: HashMap<usize, ImageView>,
    samplers: HashMap<Option<usize>, Sampler>,
    materials: HashMap<Option<usize>, Material>,
    skins: HashMap<usize, Skin>,
}

impl Asset for GltfAsset {
    type Error = GltfLoadingError;
    type Builder = Graphics;
    type Decoded = GltfDecoded;

    fn build(decoded: GltfDecoded, graphics: &mut Graphics) -> Result<Self, GltfLoadingError> {
        let mut ctx = GltfBuildContext {
            decoded: &decoded,
            graphics,
            images: HashMap::new(),
            samplers: HashMap::new(),
            materials: HashMap::new(),
            skins: HashMap::new(),
        };

        let meshes = decoded
            .gltf
            .meshes()
            .map(|mesh| ctx.create_mesh(mesh))
            .collect::<Result<_, _>>()?;

        let animations = decoded
            .gltf
            .animations()
            .map(|animation| ctx.create_animation(animation))
            .collect::<Result<_, _>>()?;

        Ok(GltfAsset {
            gltf: decoded.gltf,
            meshes,
            animations,
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

    #[error("Invalid convex shape provided")]
    InvalidConvexShape,

    #[error("View stride is less than accessor size")]
    InvalidViewStride,
}

fn align_vec(bytes: &mut Vec<u8>, align_mask: usize) {
    let new_size = (bytes.len() + align_mask) & !align_mask;
    bytes.resize(new_size, 0xfe);
}

fn read_accessor<'a>(
    accessor: gltf::Accessor<'_>,
    decoded: &'a GltfDecoded,
) -> Result<(&'a [u8], usize), GltfLoadingError> {
    let view = accessor
        .view()
        .ok_or(GltfLoadingError::SparseAccessorUnsupported)?;

    let stride = view.stride().unwrap_or(accessor.size());
    if stride < accessor.size() {
        tracing::error!(
            "Accessor '{}' with size '{}' is bound to view '{}' with insufficient stride '{}'",
            accessor.index(),
            accessor.size(),
            view.index(),
            stride,
        );
        return Err(GltfLoadingError::InvalidViewStride);
    }

    // Total byte count for accessor.
    let accessor_size = if accessor.count() == 0 {
        0
    } else {
        (accessor.count() - 1) * stride + accessor.size()
    };

    if view.length() < accessor_size + accessor.offset() {
        tracing::error!(
            "Accessor '{}' is out of buffer view bounds",
            accessor.index(),
        );
        return Err(GltfLoadingError::AccessorOutOfBound);
    }

    let bytes = match view.buffer().source() {
        gltf::buffer::Source::Bin => decoded.gltf.blob.as_deref().ok_or_else(|| {
            tracing::error!("View '{}' has non-existent bin", view.index());
            GltfLoadingError::MissingSource
        })?,
        gltf::buffer::Source::Uri(uri) => decoded.sources.get(uri).ok_or_else(|| {
            tracing::error!("View '{}' has non-existent source {}", view.index(), uri);
            GltfLoadingError::MissingSource
        })?,
    };

    if bytes.len() < view.offset() + view.length() {
        tracing::error!("View '{}' is out of buffer bounds", view.index(),);
        return Err(GltfLoadingError::ViewOutOfBound);
    }

    let bytes = &bytes[view.offset() + accessor.offset()..][..accessor_size];
    Ok((bytes, stride))
}
