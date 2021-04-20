use {
    crate::{event::Event, funnel::Funnel, resources::Res},
    hecs::{Entity, World},
};

pub trait Command: Sized + Send + Sync + 'static {
    type Queue: Default + Send + Sync + 'static;

    fn enque(self, queue: &mut Self::Queue);
}

/// Controls entity based on input events.
pub struct InputController<T> {
    controls: Option<Entity>,
    translator: Box<dyn InputTranslator<T>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AssumeControlError {
    #[error("Entity does not exist")]
    NoSuchEntity,

    #[error("Entity is already controlled")]
    AlreadyControlled,
}

pub struct Controlled;

impl<T> InputController<T>
where
    T: Command,
{
    /// Create [`InputControl`] with no controlled entity.
    pub fn new(translator: impl InputTranslator<T>) -> Self {
        InputController {
            controls: None,
            translator: Box::new(translator),
        }
    }

    /// Create [`InputControl`] with controlled entity.
    pub fn with_controlled(
        entity: Entity,
        world: &mut World,
        translator: impl InputTranslator<T>,
    ) -> Result<Self, AssumeControlError> {
        match world.get::<Controlled>(entity) {
            Ok(_) => return Err(AssumeControlError::AlreadyControlled),
            Err(hecs::ComponentError::NoSuchEntity) => {
                return Err(AssumeControlError::NoSuchEntity)
            }
            Err(hecs::ComponentError::MissingComponent(_)) => {}
        }

        debug_assert!(matches!(
            world.get::<T::Queue>(entity),
            Err(hecs::ComponentError::MissingComponent(_))
        ));

        world
            .insert(entity, (Controlled, T::Queue::default()))
            .unwrap();
        Ok(InputController {
            controls: Some(entity),
            translator: Box::new(translator),
        })
    }
}

/// Event to command translator.
pub trait InputTranslator<T>: 'static {
    /// Translates event into a command.
    fn translate_event(&mut self, event: Event) -> Result<T, Event>;
}

impl<T> Funnel<Event> for InputController<T>
where
    T: Command,
{
    fn filter(&mut self, _res: &mut Res, world: &mut World, event: Event) -> Option<Event> {
        let mut queue = match self.controls {
            Some(entity) => match world.get_mut::<T::Queue>(entity) {
                Ok(queue) => queue,
                Err(_) => {
                    self.controls = None;
                    return Some(event);
                }
            },
            None => {
                return Some(event);
            }
        };

        match self.translator.translate_event(event) {
            Ok(command) => {
                command.enque(&mut queue);
                None
            }
            Err(event) => Some(event),
        }
    }
}
