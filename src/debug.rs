use {
    edict::{entity::EntityId, world::World},
    std::fmt::{self, Display},
};

/// A piece of information that can be added on entity creation.
/// Useful to print in logs along with entity id to describe the entity.
#[derive(Debug)]
pub struct DebugInfo {
    name: Box<str>,
    description: Option<Box<str>>,
}

impl DebugInfo {
    pub fn for_entity(&self, entity: EntityId) -> EntityDebugInfo<'_> {
        EntityDebugInfo {
            entity,
            name: &*self.name,
            description: self.description.as_deref(),
        }
    }
}

pub struct EntityDebugInfo<'a> {
    entity: EntityId,
    name: &'a str,
    description: Option<&'a str>,
}

impl Display for EntityDebugInfo<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (fmt.alternate(), self.description) {
            (true, Some(description)) => write!(
                fmt,
                "{{ {} : {} - {} }}",
                self.name, self.entity, description
            ),
            _ => write!(fmt, "{{ {} : {} }}", self.name, self.entity),
        }
    }
}

pub struct EntityRefDebugInfo<'a> {
    entity: EntityId,
    info: &'a DebugInfo,
}

impl<'a> EntityRefDebugInfo<'a> {
    pub fn new(entity: EntityId, info: &'a DebugInfo) -> Self {
        EntityRefDebugInfo { entity, info }
    }

    pub fn fetch(entity: EntityId, world: &'a World) -> Option<Self> {
        let info = world.query_one::<&DebugInfo>(&entity).ok()?;
        Some(EntityRefDebugInfo { entity, info })
    }
}

impl Display for EntityRefDebugInfo<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (fmt.alternate(), &self.info.description) {
            (true, Some(description)) => {
                write!(
                    fmt,
                    "{{ {} : {} - {} }}",
                    self.info.name, self.entity, description
                )
            }

            _ => write!(fmt, "{{ {} : {} }}", self.info.name, self.entity),
        }
    }
}

pub trait WorldExt {
    fn entity_display(&self, entity: EntityId) -> Option<EntityRefDebugInfo<'_>>;
}

impl WorldExt for World {
    fn entity_display(&self, entity: EntityId) -> Option<EntityRefDebugInfo<'_>> {
        EntityRefDebugInfo::fetch(entity, self)
    }
}

pub trait EntityDisplay {
    fn display<'a>(&self, info: &'a DebugInfo) -> EntityDebugInfo<'a>;
}

impl EntityDisplay for EntityId {
    fn display<'a>(&self, info: &'a DebugInfo) -> EntityDebugInfo<'a> {
        info.for_entity(*self)
    }
}
