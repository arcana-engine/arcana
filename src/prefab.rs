use {
    crate::{assets::Loader, graphics::Graphics, resources::Res},
    flume::{unbounded, Receiver, Sender},
    hecs::{Entity, World},
    std::future::Future,
    tracing::Instrument,
};

/// An prefab type that can be spawned in the [`World`].
///
/// Typical prefab would load assets and construct components from them.
pub trait Prefab: Send + Sync + 'static {
    /// Decoded representation of this prefab.
    type Loaded: Send + Sync;

    /// Prefab components loading future.
    type Fut: Future<Output = Self::Loaded> + Send;

    /// Loads prefab components.
    fn load(&self, loader: &Loader) -> Self::Fut;

    /// Spawns prefab instance using loaded components.
    fn spawn(
        loaded: Self::Loaded,
        res: &mut Res,
        world: &mut World,
        graphics: &mut Graphics,
        entity: Entity,
    ) -> eyre::Result<()>;
}

pub struct PrefabLoader {
    loader: Loader,
    sender: Sender<Box<dyn FnOnce(&mut Res, &mut World, &mut Graphics) + Send>>,
}

pub struct PrefabSpawner {
    receiver: Receiver<Box<dyn FnOnce(&mut Res, &mut World, &mut Graphics) + Send>>,
}

impl PrefabLoader {
    pub fn load_prefab<P: Prefab>(&self, prefab: P, world: &mut World) -> Entity {
        let fut = prefab.load(&self.loader);
        let entity = world.spawn((prefab,));

        let sender = self.sender.clone();
        tokio::spawn(
            async move {
                let loaded = fut.await;
                tracing::error!("Prefab loaded");

                let _ = sender.send(Box::new(move |res, world, graphics| {
                    if let Err(err) = P::spawn(loaded, res, world, graphics, entity) {
                        tracing::error!("Failed to spawn prefab: {}", err);
                        let _ = world.despawn(entity);
                    }
                }));
            }
            .in_current_span(),
        );

        entity
    }
}

impl PrefabSpawner {
    pub fn flush(&mut self, res: &mut Res, world: &mut World, graphics: &mut Graphics) {
        self.receiver.drain().for_each(|f| {
            f(res, world, graphics);
            tracing::error!("Prefab spawned");
        })
    }
}

pub fn prefab_pipe(loader: Loader) -> (PrefabLoader, PrefabSpawner) {
    let (sender, receiver) = unbounded();

    (PrefabLoader { loader, sender }, PrefabSpawner { receiver })
}
