use {
    crate::{
        event::{DeviceEvent, DeviceId, Event},
        funnel::Funnel,
        resources::Res,
    },
    hecs::{Entity, World},
    std::collections::hash_map::{Entry, HashMap},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AssumeControlError {
    /// Failed to assume control of non-existing entity.
    #[error("Failed to assume control of non-existing entity ({entity:?})")]
    NoSuchEntity { entity: Entity },

    /// Entity is already controlled
    #[error("Entity ({entity:?}) is already controlled")]
    AlreadyControlled { entity: Entity },

    /// Device is already used by controller
    #[error("Device ({device_id:?}) is already used by controller")]
    DeviceUsed { device_id: DeviceId },
}

/// Marker component. Marks that entity is being controlled.
pub struct Controlled {
    // Forbid construction outside of this module.
    __: (),
}

const CONTROLLED: Controlled = Controlled { __: () };

/// Result of `InputController::control` method
pub enum ControlResult {
    /// Event was consumed by the controller.
    /// It should not be propagated further.
    Consumed,

    /// Event ignored.
    /// It should be propagated further.
    Ignored,

    /// Controller detached from entity.
    /// Event should be propagated further.
    ControlLost,
}

/// A controller.
pub trait InputController: 'static {
    /// Component that receives command from this controller.
    type Controlled: Send + Sync + 'static;

    /// Create new component for command receiving.
    /// Called once control of an entity is assumed.
    fn controlled(&self) -> Self::Controlled;

    /// Translates device event into controls.
    fn control(&mut self, event: DeviceEvent, controlled: &mut Self::Controlled) -> ControlResult;
}

/// Collection of entity controllers mapped to device id.
pub struct Control {
    global: Option<Box<dyn ControllerEntryErased>>,
    devices: HashMap<DeviceId, Box<dyn ControllerEntryErased>>,
}

impl Control {
    pub fn new() -> Control {
        Control {
            global: None,
            devices: HashMap::new(),
        }
    }

    pub fn assume_control(
        &mut self,
        entity: Entity,
        controller: impl InputController,
        world: &mut World,
    ) -> Result<(), AssumeControlError> {
        match world.get::<Controlled>(entity).map(|_| ()) {
            Ok(()) => Err(AssumeControlError::AlreadyControlled { entity }),
            Err(hecs::ComponentError::MissingComponent(_)) => {
                world
                    .insert(entity, (CONTROLLED, controller.controlled()))
                    .unwrap();
                self.global = Some(Box::new(ControllerEntry { entity, controller }));
                Ok(())
            }
            Err(hecs::ComponentError::NoSuchEntity) => {
                Err(AssumeControlError::NoSuchEntity { entity })
            }
        }
    }

    pub fn assume_device_control(
        &mut self,
        device_id: DeviceId,
        entity: Entity,
        controller: impl InputController,
        world: &mut World,
    ) -> Result<(), AssumeControlError> {
        match self.devices.entry(device_id) {
            Entry::Occupied(_) => Err(AssumeControlError::DeviceUsed { device_id }),
            Entry::Vacant(entry) => match world.get::<Controlled>(entity).map(|_| ()) {
                Ok(()) => Err(AssumeControlError::AlreadyControlled { entity }),
                Err(hecs::ComponentError::MissingComponent(_)) => {
                    world
                        .insert(entity, (CONTROLLED, controller.controlled()))
                        .unwrap();
                    entry.insert(Box::new(ControllerEntry { entity, controller }));
                    Ok(())
                }
                Err(hecs::ComponentError::NoSuchEntity) => {
                    Err(AssumeControlError::NoSuchEntity { entity })
                }
            },
        }
    }
}

struct ControllerEntry<T> {
    entity: Entity,
    controller: T,
}

trait ControllerEntryErased {
    fn control(&mut self, world: &mut World, event: DeviceEvent) -> ControlResult;
}

impl<T> ControllerEntryErased for ControllerEntry<T>
where
    T: InputController,
{
    fn control(&mut self, world: &mut World, event: DeviceEvent) -> ControlResult {
        match world.query_one_mut::<(Option<&Controlled>, Option<&mut T::Controlled>)>(self.entity)
        {
            Ok((None, None)) => {
                // Both control components were removed.
                ControlResult::ControlLost
            }
            Ok((None, Some(_))) => {
                let _ = world.remove_one::<T::Controlled>(self.entity);
                ControlResult::ControlLost
            }
            Ok((Some(_), None)) => {
                let _ = world.remove_one::<Controlled>(self.entity);
                ControlResult::ControlLost
            }
            Ok((Some(_), Some(controlled))) => match self.controller.control(event, controlled) {
                ControlResult::Consumed => ControlResult::Consumed,
                ControlResult::Ignored => ControlResult::Ignored,
                ControlResult::ControlLost => {
                    world
                        .remove::<(Controlled, T::Controlled)>(self.entity)
                        .unwrap();
                    ControlResult::ControlLost
                }
            },
            Err(_) => {
                // Entity was despawned, as it is impossible to not satisfy pair of options query.
                ControlResult::ControlLost
            }
        }
    }
}

impl Funnel<Event> for Control {
    fn filter(&mut self, _res: &mut Res, world: &mut World, event: Event) -> Option<Event> {
        match event {
            Event::DeviceEvent { device_id, event } => {
                let event_opt = match self.devices.get_mut(&device_id) {
                    Some(controller) => match controller.control(world, event.clone()) {
                        ControlResult::ControlLost => {
                            self.devices.remove(&device_id);
                            Some(event)
                        }
                        ControlResult::Consumed => None,
                        ControlResult::Ignored => Some(event),
                    },
                    None => Some(event),
                };

                let event_opt = match (event_opt, &mut self.global) {
                    (Some(event), Some(controller)) => {
                        match controller.control(world, event.clone()) {
                            ControlResult::ControlLost => {
                                self.global = None;
                                Some(event)
                            }
                            ControlResult::Consumed => None,
                            ControlResult::Ignored => Some(event),
                        }
                    }
                    (event, _) => event,
                };

                event_opt.map(|event| Event::DeviceEvent { device_id, event })
            }
            _ => Some(event),
        }
    }
}
