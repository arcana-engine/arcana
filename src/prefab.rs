use {
    crate::{graphics::Graphics, resources::Res},
    flume::{unbounded, Receiver, Sender},
    goods::{Asset, AssetResult, Loader},
    hecs::{Entity, World},
    tracing::Instrument,
    uuid::Uuid,
};

/// An prefab type that can be spawned in the [`World`].
///
/// Typical prefab would load assets and construct components from them.
pub trait Prefab: Asset {
    /// Spawns prefab instance using loaded components.
    fn insert(
        result: AssetResult<Self>,
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
    pub fn spawn_prefab<P: Prefab>(&self, uuid: &Uuid, world: &mut World) -> Entity {
        let handle = self.loader.load::<P>(uuid);
        let entity = world.reserve_entity();

        let sender = self.sender.clone();
        tokio::spawn(
            async move {
                let result = handle.await;
                let _ = sender.send(Box::new(move |res, world, graphics| {
                    if let Err(err) = P::insert(result, res, world, graphics, entity) {
                        tracing::error!("Failed to spawn prefab: {}", err);
                        let _ = world.despawn(entity);
                    }
                }));
            }
            .in_current_span(),
        );

        entity
    }

    pub fn load_prefab<P: Prefab>(&self, entity: Entity, uuid: &Uuid, world: &mut World) -> Entity {
        let handle = self.loader.load::<P>(uuid);

        let sender = self.sender.clone();
        tokio::spawn(
            async move {
                let result = handle.await;
                let _ = sender.send(Box::new(move |res, world, graphics| {
                    if let Err(err) = P::insert(result, res, world, graphics, entity) {
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
        })
    }
}

pub fn prefab_pipe(loader: Loader) -> (PrefabLoader, PrefabSpawner) {
    let (sender, receiver) = unbounded();

    (PrefabLoader { loader, sender }, PrefabSpawner { receiver })
}
