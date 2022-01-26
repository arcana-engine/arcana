use std::any::TypeId;

use goods::{AssetBuild, AssetHandle, AssetId, Loader};
use hashbrown::{hash_map::Entry, HashMap};

enum AssetState<A> {
    Requested {
        handle: AssetHandle<A>,
        polled: bool,
    },
    Loaded {
        asset: A,
    },
    Error {
        err: goods::Error,
    },
}

pub(super) struct AssetCache<A> {
    assets: HashMap<AssetId, AssetState<A>>,
}

impl<A> AssetCache<A> {
    pub fn new() -> Self {
        AssetCache {
            assets: HashMap::new(),
        }
    }

    pub fn build<B>(&mut self, id: AssetId, loader: &Loader, builder: &mut B) -> Option<&A>
    where
        A: AssetBuild<B>,
    {
        match self.assets.entry(id) {
            Entry::Occupied(mut entry) => match entry.get_mut() {
                AssetState::Loaded { .. } => match entry.into_mut() {
                    AssetState::Loaded { asset } => Some(asset),
                    _ => unreachable!(),
                },
                AssetState::Requested {
                    handle,
                    polled: polled @ false,
                } => {
                    *polled = true;
                    match handle.get_ready() {
                        None => None,
                        Some(mut result) => match result.build(builder) {
                            Ok(asset) => {
                                let asset = asset.clone();
                                entry.insert(AssetState::Loaded { asset });
                                match entry.into_mut() {
                                    AssetState::Loaded { asset } => Some(asset),
                                    _ => unreachable!(),
                                }
                            }
                            Err(err) => {
                                tracing::error!(
                                    "Failed to load asset {}: {}. {:#}",
                                    id,
                                    std::any::type_name::<A>(),
                                    err
                                );
                                entry.insert(AssetState::Error { err });
                                None
                            }
                        },
                    }
                }
                _ => None,
            },
            Entry::Vacant(entry) => {
                let mut handle = loader.load::<A, _>(id);

                match handle.get_ready() {
                    None => {
                        entry.insert(AssetState::Requested {
                            handle,
                            polled: true,
                        });
                        None
                    }
                    Some(mut result) => match result.build(builder) {
                        Ok(asset) => {
                            let asset = asset.clone();
                            let state = entry.insert(AssetState::Loaded { asset });
                            match state {
                                AssetState::Loaded { asset } => Some(asset),
                                _ => unreachable!(),
                            }
                        }
                        Err(err) => {
                            entry.insert(AssetState::Error { err });
                            None
                        }
                    },
                }
            }
        }
    }

    pub fn cleanup(&mut self) {
        self.assets.retain(|id, state| match state {
            AssetState::Requested { polled, .. } => {
                *polled = false;
                true
            }
            AssetState::Loaded { .. } => false,
            AssetState::Error { err } => {
                true
            }
        })
    }
}

pub(super) trait AnyAssetCache: Send + Sync {
    fn type_id(&self) -> TypeId;

    fn cleanup(&mut self);
}

impl<A> AnyAssetCache for AssetCache<A>
where
    A: Send + Sync + 'static,
{
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn cleanup(&mut self) {
        self.cleanup();
    }
}

impl dyn AnyAssetCache {
    pub fn cast<A: 'static>(&mut self) -> &mut AssetCache<A> {
        debug_assert_eq!(self.type_id(), TypeId::of::<AssetCache<A>>());
        unsafe { &mut *(self as *mut dyn AnyAssetCache as *mut AssetCache<A>) }
    }
}
