use std::collections::hash_map::{Entry, HashMap};

use hecs::{Entity, World};

use crate::{
    command::CommandQueue,
    event::{
        AxisId, ButtonId, DeviceEvent, DeviceId, ElementState, Event, KeyboardInput, MouseButton,
        MouseScrollDelta, WindowEvent,
    },
    funnel::Funnel,
    resources::Res,
};

#[derive(Clone, Copy, Debug)]
pub enum InputEvent {
    CursorMoved {
        position: (f64, f64),
    },
    CursorEntered,
    CursorLeft,
    MouseMotion {
        delta: (f64, f64),
    },
    MouseWheel {
        delta: MouseScrollDelta,
    },
    MouseInput {
        state: ElementState,
        button: MouseButton,
    },
    KeyboardInput(KeyboardInput),
    Motion {
        axis: AxisId,
        value: f64,
    },
    Button {
        button: ButtonId,
        state: ElementState,
    },
}

/// Device is already associated with a controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Device ({device_id:?}) is already associated with a controller")]
pub struct DeviceUsed {
    device_id: DeviceId,
}

/// Result of `InputController::control` method
pub enum ControlResult {
    /// Event was consumed by the controller.
    /// It should not be propagated further.
    Consumed,

    /// Event ignored.
    /// It should be propagated further.
    Ignored,

    /// Controller detached and should be removed.
    /// Event should be propagated further.
    ControlLost,
}

/// An input controller.
/// Receives device events from `Control` hub.
pub trait InputController: Send + 'static {
    /// Translates device event into controls.
    fn control(&mut self, event: InputEvent, res: &mut Res, world: &mut World) -> ControlResult;
}

/// Collection of controllers.
pub struct Control {
    /// Controllers bound to specific devices.
    devices: HashMap<DeviceId, Box<dyn InputController>>,

    /// Global controller that receives all events unhandled by device specific controllers.
    global: slab::Slab<Box<dyn InputController>>,
}

/// Identifier of the controller set in global slot.
/// See [`Control::add_global_controller`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GlobalControllerId {
    idx: usize,
}

impl Control {
    /// Returns empty collection of controllers
    pub fn new() -> Control {
        Control {
            devices: HashMap::new(),
            global: slab::Slab::new(),
        }
    }

    /// Assign global controller.
    pub fn add_global_controller(
        &mut self,
        controller: impl InputController,
    ) -> GlobalControllerId {
        let idx = self.global.insert(Box::new(controller));
        GlobalControllerId { idx }
    }

    /// Assign global controller to specific device.
    pub fn set_device_control(
        &mut self,
        device_id: DeviceId,
        controller: impl InputController,
    ) -> Result<(), DeviceUsed> {
        match self.devices.entry(device_id) {
            Entry::Occupied(_) => Err(DeviceUsed { device_id }),
            Entry::Vacant(entry) => {
                entry.insert(Box::new(controller));
                Ok(())
            }
        }
    }
}
impl Funnel<Event> for Control {
    fn filter(&mut self, res: &mut Res, world: &mut World, event: Event) -> Option<Event> {
        let (input_event, device_id) = match event {
            Event::DeviceEvent {
                device_id,
                event: ref device_event,
            } => {
                let input_event = match device_event {
                    &DeviceEvent::Motion { axis, value } => InputEvent::Motion { axis, value },
                    &DeviceEvent::MouseMotion { delta } => InputEvent::MouseMotion { delta },
                    &DeviceEvent::MouseWheel { delta } => InputEvent::MouseWheel { delta },
                    &DeviceEvent::Button { button, state } => InputEvent::Button { button, state },
                    _ => return Some(event),
                };
                (input_event, device_id)
            }

            Event::WindowEvent {
                event: ref window_event,
                ..
            } => match window_event {
                &WindowEvent::MouseInput {
                    device_id,
                    button,
                    state,
                    ..
                } => (InputEvent::MouseInput { state, button }, device_id),
                &WindowEvent::KeyboardInput {
                    device_id, input, ..
                } => (InputEvent::KeyboardInput(input), device_id),
                _ => return Some(event),
            },
            _ => return Some(event),
        };

        let mut event_opt = match self.devices.get_mut(&device_id) {
            Some(controller) => match controller.control(input_event, res, world) {
                ControlResult::ControlLost => {
                    self.devices.remove(&device_id);
                    Some(event)
                }
                ControlResult::Consumed => None,
                ControlResult::Ignored => Some(event),
            },
            None => Some(event),
        };

        for idx in 0..self.global.len() {
            if let Some(event) = event_opt.take() {
                if let Some(controller) = self.global.get_mut(idx) {
                    match controller.control(input_event, res, world) {
                        ControlResult::ControlLost => {
                            self.global.remove(idx);
                            event_opt = Some(event);
                        }
                        ControlResult::Consumed => {}
                        ControlResult::Ignored => event_opt = Some(event),
                    }
                }
            } else {
                break;
            }
        }

        event_opt
    }
}

/// Translates device events into commands and
pub trait EventTranslator {
    type Command;

    fn translate(&mut self, event: InputEvent) -> Option<Self::Command>;
}

/// Error that can occur when assuming control over an entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AssumeControlError {
    /// Failed to assume control of non-existing entity.
    #[error("Failed to assume control of non-existing entity ({entity:?})")]
    NoSuchEntity { entity: Entity },

    /// Entity is already controlled
    #[error("Entity ({entity:?}) is already controlled")]
    AlreadyControlled { entity: Entity },
}

/// Marker component. Marks that entity is being controlled.
pub struct Controlled {
    // Forbid construction outside of this module.
    __: (),
}

const CONTROLLED: Controlled = Controlled { __: () };

/// A kind of [`InputController`]s that yield commands and sends them to a command queue of an entity.
pub struct EntityController<T> {
    commander: T,
    entity: Entity,
}

impl<T> EntityController<T>
where
    T: EventTranslator,
    T::Command: Send + Sync + 'static,
{
    pub fn assume_control(
        commander: T,
        entity: Entity,
        world: &mut World,
    ) -> Result<Self, AssumeControlError> {
        match world.query_one_mut::<&Controlled>(entity).is_ok() {
            true => Err(AssumeControlError::AlreadyControlled { entity }),
            false => {
                world
                    .insert(entity, (CONTROLLED, CommandQueue::<T::Command>::new()))
                    .map_err(|hecs::NoSuchEntity| AssumeControlError::NoSuchEntity { entity })?;
                Ok(EntityController { commander, entity })
            }
        }
    }
}

impl<T> InputController for EntityController<T>
where
    T: EventTranslator + Send + 'static,
    T::Command: Send + Sync + 'static,
{
    fn control(&mut self, event: InputEvent, _res: &mut Res, world: &mut World) -> ControlResult {
        match world.query_one_mut::<&mut CommandQueue<T::Command>>(self.entity) {
            Ok(queue) => match self.commander.translate(event) {
                None => ControlResult::Ignored,
                Some(command) => {
                    queue.add(command);
                    ControlResult::Consumed
                }
            },
            Err(_err) => ControlResult::ControlLost,
        }
    }
}
