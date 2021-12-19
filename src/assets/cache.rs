use std::{marker::PhantomData, mem::take};

use goods::{Asset, AssetBuild, AssetHandle, AssetId, AssetResult, Loader};
use hashbrown::{hash_map::Entry, HashMap};

use crate::{
    system::{System, SystemContext},
    task::{with_async_task_context, Spawner, TaskContext},
};

pub struct AssetLoadCache<A: Asset> {
    to_load: Vec<(AssetId, Option<AssetHandle<A>>, Option<AssetResult<A>>)>,
    loaded: HashMap<AssetId, Option<A>>,
    task_running: bool,
}

impl<A> AssetLoadCache<A>
where
    A: Asset,
{
    pub fn new() -> Self {
        AssetLoadCache {
            to_load: Vec::new(),
            loaded: HashMap::new(),
            task_running: false,
        }
    }

    pub fn load(&mut self, id: AssetId, loader: &Loader) {
        match self.loaded.entry(id) {
            Entry::Occupied(_) => return,
            Entry::Vacant(entry) => {
                let handle = loader.load(id);
                entry.insert(None);
                self.to_load.push((id, Some(handle), None));
            }
        }
    }

    pub fn get_ready(&self, id: AssetId) -> Option<&A> {
        self.loaded.get(&id).and_then(Option::as_ref)
    }

    pub fn get_or_load(&mut self, id: AssetId, loader: &Loader) -> Option<&A> {
        match self.loaded.entry(id) {
            Entry::Occupied(entry) => match entry.into_mut() {
                None => None,
                Some(asset) => Some(asset),
            },
            Entry::Vacant(entry) => {
                let handle = loader.load(id);
                entry.insert(None);
                self.to_load.push((id, Some(handle), None));
                None
            }
        }
    }

    pub fn ensure_task<B, F>(&mut self, spawner: &mut Spawner, builder: F)
    where
        A: AssetBuild<B>,
        B: 'static,
        F: Fn(TaskContext<'_>) -> &mut B + Send + 'static,
    {
        if self.to_load.is_empty() || self.task_running {
            return;
        }

        self.task_running = true;
        spawner.spawn(async move {
            let mut to_load = Vec::new();

            loop {
                debug_assert!(to_load.is_empty());

                let run = with_async_task_context(|cx| {
                    let me = cx.res.get_mut::<Self>().unwrap();

                    if me.to_load.is_empty() {
                        // If there's noting to load - end task.
                        me.task_running = false;
                        return false;
                    }

                    // Or take all sets to load into async scope.
                    to_load = take(&mut me.to_load);
                    true
                });

                if !run {
                    break;
                }

                // Ensure all map assets are loaded.
                for (_, handle, result) in &mut to_load {
                    debug_assert!(handle.is_some());
                    debug_assert!(result.is_none());
                    *result = Some(handle.take().unwrap().await);
                }

                with_async_task_context(|mut cx| {
                    for (id, handle, result) in to_load.drain(..) {
                        debug_assert!(result.is_some());
                        debug_assert!(handle.is_none());

                        let mut result = result.unwrap();
                        let set = result.build(builder(cx.reborrow()));

                        match set {
                            Ok(set) => {
                                let me = cx.res.get_mut::<Self>().unwrap();
                                me.loaded.insert(id, Some(set.clone()));
                            }
                            Err(err) => {
                                tracing::error!("Failed to load set '{}': {:#}", id, err);
                            }
                        }
                    }
                });
            }

            Ok(())
        });
    }

    pub fn clear_ready(&mut self) {
        self.loaded.retain(|_, opt| opt.is_none());
    }
}

pub struct AssetLoadCacheClearSystem<A> {
    name: Box<str>,
    marker: PhantomData<fn(A) -> A>,
}

impl<A> AssetLoadCacheClearSystem<A>
where
    A: Asset,
{
    pub fn new() -> Self {
        AssetLoadCacheClearSystem {
            name: format!("Asset load cache clear system for '{}'", A::name()).into_boxed_str(),
            marker: PhantomData,
        }
    }
}

impl<A> System for AssetLoadCacheClearSystem<A>
where
    A: Asset,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        if let Some(cache) = cx.res.get_mut::<AssetLoadCache<A>>() {
            cache.clear_ready();
        }

        Ok(())
    }
}
