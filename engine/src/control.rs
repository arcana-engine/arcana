use std::{
    collections::hash_map::{Entry, HashMap},
    fmt::Debug,
    hash::Hash,
    ops::Neg,
};

use edict::{
    prelude::{Component, EntityId, World},
    world::NoSuchEntity,
};
use winit::event::VirtualKeyCode;

use crate::{
    command::CommandQueue,
    event::{
        AxisId, ButtonId, DeviceEvent, DeviceId, ElementState, Event, KeyboardInput, MouseButton,
        MouseScrollDelta, WindowEvent,
    },
    funnel::Funnel,
};

#[derive(Clone, Copy, Debug)]
pub enum InputEvent {
    Focused(bool),
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
    fn control(&mut self, event: InputEvent, world: &World) -> ControlResult;
}

impl<F> InputController for F
where
    F: FnMut(InputEvent, &World) -> ControlResult + Send + 'static,
{
    fn control(&mut self, event: InputEvent, world: &World) -> ControlResult {
        (*self)(event, world)
    }
}

/// Collection of controllers.
#[derive(Default)]
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
        Control::default()
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

pub struct ControlFunnel;

impl Funnel<Event> for ControlFunnel {
    fn filter(&mut self, world: &mut World, event: Event) -> Option<Event> {
        let mut control = world.expect_resource_mut::<Control>();

        let (input_event, device_id) = match event {
            Event::DeviceEvent {
                device_id,
                event: ref device_event,
            } => {
                let input_event = match *device_event {
                    DeviceEvent::Motion { axis, value } => InputEvent::Motion { axis, value },
                    DeviceEvent::MouseMotion { delta } => InputEvent::MouseMotion { delta },
                    DeviceEvent::MouseWheel { delta } => InputEvent::MouseWheel { delta },
                    DeviceEvent::Button { button, state } => InputEvent::Button { button, state },
                    _ => return Some(event),
                };
                (input_event, device_id)
            }

            Event::WindowEvent {
                event: ref window_event,
                ..
            } => match *window_event {
                WindowEvent::MouseInput {
                    device_id,
                    button,
                    state,
                    ..
                } => (InputEvent::MouseInput { state, button }, device_id),
                WindowEvent::KeyboardInput {
                    device_id, input, ..
                } => (InputEvent::KeyboardInput(input), device_id),
                WindowEvent::CursorMoved {
                    device_id,
                    position,
                    ..
                } => (
                    InputEvent::CursorMoved {
                        position: (position.x, position.y),
                    },
                    device_id,
                ),
                WindowEvent::Focused(v) => {
                    // This event is always broadcast to every controller.
                    let mut device_id_control_lost = Vec::new();
                    for (device_id, controller) in &mut control.devices {
                        if let ControlResult::ControlLost =
                            controller.control(InputEvent::Focused(v), world)
                        {
                            device_id_control_lost.push(*device_id);
                        }
                    }

                    for device_id in device_id_control_lost {
                        control.devices.remove(&device_id);
                    }

                    let mut global_control_lost = Vec::new();
                    for (idx, controller) in control.global.iter_mut() {
                        if let ControlResult::ControlLost =
                            controller.control(InputEvent::Focused(v), world)
                        {
                            global_control_lost.push(idx);
                        }
                    }

                    for idx in global_control_lost {
                        control.global.remove(idx);
                    }

                    return Some(event);
                }

                _ => return Some(event),
            },
            _ => return Some(event),
        };

        let mut consumed = match control.devices.get_mut(&device_id) {
            Some(controller) => match controller.control(input_event, world) {
                ControlResult::ControlLost => {
                    control.devices.remove(&device_id);
                    false
                }
                ControlResult::Consumed => true,
                ControlResult::Ignored => false,
            },
            None => false,
        };

        for idx in 0..control.global.len() {
            if !consumed {
                if let Some(controller) = control.global.get_mut(idx) {
                    match controller.control(input_event, world) {
                        ControlResult::ControlLost => {
                            control.global.remove(idx);
                        }
                        ControlResult::Consumed => consumed = true,
                        ControlResult::Ignored => {}
                    }
                }
            } else {
                break;
            }
        }

        if !consumed {
            Some(event)
        } else {
            None
        }
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
    NoSuchEntity { entity: EntityId },

    /// EntityId is already controlled
    #[error("EntityId ({entity:?}) is already controlled")]
    AlreadyControlled { entity: EntityId },
}

/// Marker component. Marks that entity is being controlled.
#[derive(Component)]
pub struct Controlled {
    // Forbid construction outside of this module.
    __: (),
}

const CONTROLLED: Controlled = Controlled { __: () };

/// A kind of [`InputController`]s that yield commands and sends them to a command queue of an entity.
pub struct EntityController<T> {
    commander: T,
    entity: EntityId,
}

impl<T> EntityController<T>
where
    T: EventTranslator,
    T::Command: Send + Sync + 'static,
{
    pub fn assume_control(
        commander: T,
        entity: EntityId,
        world: &mut World,
    ) -> Result<Self, AssumeControlError> {
        match world.query_one::<&Controlled>(entity).is_ok() {
            true => Err(AssumeControlError::AlreadyControlled { entity }),
            false => {
                world
                    .insert_bundle(entity, (CONTROLLED, CommandQueue::<T::Command>::new()))
                    .map_err(|NoSuchEntity| AssumeControlError::NoSuchEntity { entity })?;
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
    fn control(&mut self, event: InputEvent, world: &World) -> ControlResult {
        let result = world.for_one::<&mut CommandQueue<T::Command>, _, _>(self.entity, |queue| {
            match self.commander.translate(event) {
                None => ControlResult::Ignored,
                Some(command) => {
                    queue.add(command);
                    ControlResult::Consumed
                }
            }
        });

        match result {
            Ok(result) => result,
            Err(_err) => ControlResult::ControlLost,
        }
    }
}

/// Basic configurable system to consume recognized key input into events.
///
/// Keys can be configured to produce events of signals.
/// When configured to produce events pressing the key would cause event to be emitted.
/// When configured to produce signal pressing the key switches state to signalling and event is emitted repeatedly until key is unpressed.
///
/// Only keys with key code can be configured.
#[derive(Clone, Debug, Default)]
pub struct SimpleKeyBinder<T> {
    bindings: HashMap<VirtualKeyCode, SimpleKeyBinding<T>>,
}

#[derive(Clone, Debug, Default)]
struct SimpleKeyBinding<T> {
    pressed: bool,
    action: SimpleKeyEventAction<T>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SimpleKeyEventAction<T> {
    #[serde(default = "none", skip_serializing_if = "Option::is_none")]
    on_press: Option<T>,

    #[serde(default = "none", skip_serializing_if = "Option::is_none")]
    on_release: Option<T>,

    #[serde(default = "none", skip_serializing_if = "Option::is_none")]
    on_hold: Option<T>,
}

fn none<T>() -> Option<T> {
    None
}

impl<T> SimpleKeyEventAction<T> {
    pub fn is_empty(&self) -> bool {
        self.on_press.is_none() && self.on_release.is_none() && self.on_hold.is_none()
    }
}

impl<T> Default for SimpleKeyEventAction<T> {
    fn default() -> Self {
        SimpleKeyEventAction {
            on_press: None,
            on_release: None,
            on_hold: None,
        }
    }
}

/// Deserialized from key mapping,
impl<'de, T> serde::Deserialize<'de> for SimpleKeyBinder<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let builder = SimpleKeyBuilder::<T>::deserialize(deserializer)?;
        Ok(SimpleKeyBinder::from_builder(builder))
    }
}

impl<T> SimpleKeyBinder<T> {
    /// Returns new builder for the key binder.
    pub fn builder() -> SimpleKeyBuilder<T> {
        SimpleKeyBuilder::new()
    }

    /// Returns new binder configured from binder builder.
    ///
    /// Action kind is fetched from action itself.
    pub fn from_builder(builder: SimpleKeyBuilder<T>) -> Self {
        SimpleKeyBinder {
            bindings: builder
                .bindings
                .into_iter()
                .map(|(key, action)| {
                    let binding = SimpleKeyBinding {
                        action,
                        pressed: false,
                    };

                    (key, binding)
                })
                .collect(),
        }
    }

    /// Returns new binder configured from binder builder.
    ///
    /// Action kind is fetched from action itself.
    ///
    /// Does not consume builder object.
    pub fn from_borrowed_builder(builder: &SimpleKeyBuilder<T>) -> Self
    where
        T: Clone,
    {
        SimpleKeyBinder {
            bindings: builder
                .bindings
                .iter()
                .map(|(key, action)| {
                    let binding = SimpleKeyBinding {
                        action: action.clone(),
                        pressed: false,
                    };

                    (*key, binding)
                })
                .collect(),
        }
    }

    /// Returns binder builder matching current binder configuration.
    pub fn to_builder(&self) -> SimpleKeyBuilder<T>
    where
        T: Clone,
    {
        SimpleKeyBuilder {
            bindings: self
                .bindings
                .iter()
                .map(|(key, binding)| (*key, binding.action.clone()))
                .collect(),
        }
    }

    /// Handle input key event.
    pub fn handle_input(&mut self, input: &KeyboardInput) -> Option<&T> {
        let binding = self.bindings.get_mut(input.virtual_keycode.as_ref()?)?;
        match input.state {
            ElementState::Pressed => {
                if binding.pressed {
                    None
                } else {
                    binding.pressed = true;
                    binding.action.on_press.as_ref()
                }
            }
            ElementState::Released => {
                if binding.pressed {
                    binding.pressed = false;
                    binding.action.on_release.as_ref()
                } else {
                    None
                }
            }
        }
    }

    /// Returns an iterator over current `on_hold` actions.
    pub fn iter_holds(&self) -> impl Iterator<Item = &T> + '_ {
        self.bindings.values().filter_map(|binding| {
            if binding.pressed {
                binding.action.on_hold.as_ref()
            } else {
                None
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SimpleKeyBuilder<T> {
    bindings: HashMap<VirtualKeyCode, SimpleKeyEventAction<T>>,
}

impl<T> Default for SimpleKeyBuilder<T> {
    fn default() -> Self {
        SimpleKeyBuilder {
            bindings: HashMap::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyAlreadyBound {
    key: VirtualKeyCode,
}

impl<T> SimpleKeyBuilder<T> {
    /// Returns new empty mapping.
    pub fn new() -> Self {
        SimpleKeyBuilder {
            bindings: HashMap::new(),
        }
    }

    /// Binds action to a key press.
    ///
    /// Panics if key press is already bound.
    pub fn on_press<A>(mut self, key: VirtualKeyCode, action: A) -> Self
    where
        A: Into<T>,
    {
        self.try_on_press(key, action).unwrap();
        self
    }

    /// Binds action to a key release.
    ///
    /// Panics if key release is already bound.
    pub fn on_release<A>(mut self, key: VirtualKeyCode, action: A) -> Self
    where
        A: Into<T>,
    {
        self.try_on_release(key, action).unwrap();
        self
    }

    /// Binds action to a key press and release.
    ///
    /// Panics if key press or release is already bound.
    pub fn on_switch<P, R>(mut self, key: VirtualKeyCode, press: P, release: R) -> Self
    where
        P: Into<T>,
        R: Into<T>,
    {
        self.try_on_switch(key, press, release).unwrap();
        self
    }

    /// Binds action to a key press and release.
    ///
    /// Panics if key press or release is already bound.
    pub fn on_switch_with<F, V, A>(
        mut self,
        key: VirtualKeyCode,
        f: F,
        press: V,
        release: V,
    ) -> Self
    where
        F: FnMut(V) -> A,
        A: Into<T>,
    {
        self.try_on_switch_with(key, f, press, release).unwrap();
        self
    }

    /// Binds action to a key press and release.
    ///
    /// Panics if key press or release is already bound.
    pub fn on_switch_inverse<F, V, A>(mut self, key: VirtualKeyCode, f: F, action: V) -> Self
    where
        F: FnMut(V) -> A,
        V: Copy + Neg<Output = V>,
        A: Into<T>,
    {
        self.try_on_switch_with(key, f, action, -action).unwrap();
        self
    }

    /// Binds action to a pair of keys with press and release actions inverted.
    ///
    /// Panics if any of the two keys press or release is already bound.
    pub fn on_key_axis<F, V, A>(
        mut self,
        forward: VirtualKeyCode,
        backward: VirtualKeyCode,
        f: F,
        action: V,
    ) -> Self
    where
        F: FnMut(V) -> A,
        V: Copy + Neg<Output = V>,
        A: Into<T>,
    {
        self.try_on_key_axis(forward, backward, f, action).unwrap();
        self
    }

    /// Binds action to a key hold.
    ///
    /// Panics if key hold is already bound.
    pub fn on_hold<A>(mut self, key: VirtualKeyCode, action: A) -> Self
    where
        A: Into<T>,
    {
        self.try_on_hold(key, action)
            .map_err(|_| "Duplicate key hold")
            .unwrap();
        self
    }

    /// Binds action to a key press.
    ///
    /// Fails if key press is already bound.
    pub fn try_on_press<A>(&mut self, key: VirtualKeyCode, action: A) -> Result<(), KeyAlreadyBound>
    where
        A: Into<T>,
    {
        let bind = self.bindings.entry(key).or_default();

        match &mut bind.on_press {
            Some(_) => Err(KeyAlreadyBound { key }),
            slot => {
                *slot = Some(action.into());
                Ok(())
            }
        }
    }

    /// Clears on press action for the key if there any.
    ///
    /// Returns some action if it was bound.
    /// Returns none if there were none.
    pub fn clear_on_press(&mut self, key: VirtualKeyCode) -> Option<T> {
        match self.bindings.entry(key) {
            Entry::Vacant(_) => None,
            Entry::Occupied(mut entry) => {
                let binding = entry.get_mut();
                let action = binding.on_press.take();
                if binding.on_release.is_none() && binding.on_hold.is_none() {
                    entry.remove();
                }
                action
            }
        }
    }

    /// Binds action to a key press and release events.
    ///
    /// Fails if key press or released is already bound.
    pub fn try_on_switch<P, R>(
        &mut self,
        key: VirtualKeyCode,
        press: P,
        release: R,
    ) -> Result<(), KeyAlreadyBound>
    where
        P: Into<T>,
        R: Into<T>,
    {
        let bind = self.bindings.entry(key).or_default();

        match (&mut bind.on_press, &mut bind.on_release) {
            (Some(_), _) | (_, Some(_)) => Err(KeyAlreadyBound { key }),
            (on_press, on_release) => {
                *on_press = Some(press.into());
                *on_release = Some(release.into());
                Ok(())
            }
        }
    }

    /// Binds action to a key press and release events.
    ///
    /// Fails if key press or released is already bound.
    pub fn try_on_switch_with<F, V, A>(
        &mut self,
        key: VirtualKeyCode,
        mut f: F,
        press: V,
        release: V,
    ) -> Result<(), KeyAlreadyBound>
    where
        F: FnMut(V) -> A,
        A: Into<T>,
    {
        let bind = self.bindings.entry(key).or_default();

        match (&mut bind.on_press, &mut bind.on_release) {
            (Some(_), _) | (_, Some(_)) => Err(KeyAlreadyBound { key }),
            (on_press, on_release) => {
                *on_press = Some(f(press).into());
                *on_release = Some(f(release).into());
                Ok(())
            }
        }
    }

    /// Binds action to a key press and release events.
    ///
    /// Fails if key press or released is already bound.
    pub fn try_on_switch_inverse<F, V, A>(
        &mut self,
        key: VirtualKeyCode,
        mut f: F,
        action: V,
    ) -> Result<(), KeyAlreadyBound>
    where
        F: FnMut(V) -> A,
        V: Copy + Neg<Output = V>,
        A: Into<T>,
    {
        let bind = self.bindings.entry(key).or_default();

        match (&mut bind.on_press, &mut bind.on_release) {
            (Some(_), _) | (_, Some(_)) => Err(KeyAlreadyBound { key }),
            (on_press, on_release) => {
                *on_press = Some(f(action).into());
                *on_release = Some(f(-action).into());
                Ok(())
            }
        }
    }

    /// Binds action to a key press and release events.
    ///
    /// Fails if key press or released is already bound.
    pub fn try_on_key_axis<F, V, A>(
        &mut self,
        forward: VirtualKeyCode,
        backward: VirtualKeyCode,
        mut f: F,
        action: V,
    ) -> Result<(), KeyAlreadyBound>
    where
        F: FnMut(V) -> A,
        V: Copy + Neg<Output = V>,
        A: Into<T>,
    {
        let forward_bind = self.bindings.entry(forward).or_default();
        match (&mut forward_bind.on_press, &mut forward_bind.on_release) {
            (Some(_), _) | (_, Some(_)) => return Err(KeyAlreadyBound { key: forward }),
            _ => {}
        }

        let backward_bind = self.bindings.entry(backward).or_default();
        match (&mut backward_bind.on_press, &mut backward_bind.on_release) {
            (Some(_), _) | (_, Some(_)) => return Err(KeyAlreadyBound { key: backward }),
            _ => {}
        }

        let forward_bind = self.bindings.entry(forward).or_default();
        forward_bind.on_press = Some(f(action).into());
        forward_bind.on_release = Some(f(-action).into());

        let backward_bind = self.bindings.entry(backward).or_default();
        backward_bind.on_press = Some(f(-action).into());
        backward_bind.on_release = Some(f(action).into());

        Ok(())
    }

    /// Binds action to a key release.
    ///
    /// Fails if key release is already bound.
    pub fn try_on_release<A>(
        &mut self,
        key: VirtualKeyCode,
        action: A,
    ) -> Result<(), KeyAlreadyBound>
    where
        A: Into<T>,
    {
        let bind = self.bindings.entry(key).or_default();

        match &mut bind.on_release {
            Some(_) => Err(KeyAlreadyBound { key }),
            slot => {
                *slot = Some(action.into());
                Ok(())
            }
        }
    }

    /// Clears on release action for the key if there any.
    ///
    /// Returns some action if it was bound.
    /// Returns none if there were none.
    pub fn clear_on_release(&mut self, key: VirtualKeyCode) -> Option<T> {
        match self.bindings.entry(key) {
            Entry::Vacant(_) => None,
            Entry::Occupied(mut entry) => {
                let binding = entry.get_mut();
                let action = binding.on_release.take();
                if binding.on_press.is_none() && binding.on_hold.is_none() {
                    entry.remove();
                }
                action
            }
        }
    }

    /// Binds action to a key hold.
    ///
    /// Fails if key hold is already bound.
    pub fn try_on_hold<A>(&mut self, key: VirtualKeyCode, action: A) -> Result<(), KeyAlreadyBound>
    where
        A: Into<T>,
    {
        let bind = self.bindings.entry(key).or_default();

        match &mut bind.on_hold {
            Some(_) => Err(KeyAlreadyBound { key }),
            slot => {
                *slot = Some(action.into());
                Ok(())
            }
        }
    }

    /// Clears on hold action for the key if there any.
    ///
    /// Returns some action if it was bound.
    /// Returns none if there were none.
    pub fn clear_on_hold(&mut self, key: VirtualKeyCode) -> Option<T> {
        match self.bindings.entry(key) {
            Entry::Vacant(_) => None,
            Entry::Occupied(mut entry) => {
                let binding = entry.get_mut();
                let action = binding.on_hold.take();
                if binding.on_press.is_none() && binding.on_release.is_none() {
                    entry.remove();
                }
                action
            }
        }
    }

    /// Converts builder into binder.
    pub fn build(self) -> SimpleKeyBinder<T> {
        SimpleKeyBinder::from_builder(self)
    }

    /// Converts builder into binder.
    ///
    /// Does not consume builder object.
    pub fn clone_build(&self) -> SimpleKeyBinder<T>
    where
        T: Clone,
    {
        SimpleKeyBinder::from_borrowed_builder(self)
    }
}
