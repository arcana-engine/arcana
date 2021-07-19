use {
    hecs::{Entity, EntityRef, World},
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
    pub fn for_entity(&self, entity: Entity) -> EntityDebugInfo<'_> {
        EntityDebugInfo {
            entity,
            name: &*self.name,
            description: self.description.as_deref(),
        }
    }
}

pub struct EntityDebugInfo<'a> {
    entity: Entity,
    name: &'a str,
    description: Option<&'a str>,
}

impl Display for EntityDebugInfo<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (fmt.alternate(), self.description) {
            (true, Some(description)) => write!(
                fmt,
                "{{ {} : {} - {} }}",
                self.name,
                self.entity.id(),
                description
            ),
            _ => write!(fmt, "{{ {} : {} }}", self.name, self.entity.id()),
        }
    }
}

pub struct EntityRefDebugInfo<'a> {
    entity: Entity,
    entity_ref: EntityRef<'a>,
}

impl<'a> EntityRefDebugInfo<'a> {
    pub fn new(entity: Entity, entity_ref: EntityRef<'a>) -> Self {
        EntityRefDebugInfo { entity, entity_ref }
    }

    pub fn fetch(entity: Entity, world: &'a World) -> Option<Self> {
        let entity_ref = world.entity(entity).ok()?;
        Some(EntityRefDebugInfo { entity, entity_ref })
    }
}

impl Display for EntityRefDebugInfo<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (
            fmt.alternate(),
            self.entity_ref.get::<DebugInfo>().as_deref(),
        ) {
            (
                true,
                Some(DebugInfo {
                    name,
                    description: Some(description),
                }),
            ) => write!(
                fmt,
                "{{ {} : {} - {} }}",
                name,
                self.entity.id(),
                description,
            ),
            (_, Some(DebugInfo { name, .. })) => {
                write!(fmt, "{{ {} : {} }}", name, self.entity.id())
            }
            (_, None) => write!(fmt, "{{ {} }}", self.entity.id()),
        }
    }
}

pub trait WorldExt {
    fn entity_display(&self, entity: Entity) -> Option<EntityRefDebugInfo<'_>>;
}

impl WorldExt for World {
    fn entity_display(&self, entity: Entity) -> Option<EntityRefDebugInfo<'_>> {
        EntityRefDebugInfo::fetch(entity, self)
    }
}

pub trait EntityDisplay {
    fn display_ref<'a>(&self, entity_ref: EntityRef<'a>) -> EntityRefDebugInfo<'a>;

    fn display<'a>(&self, info: &'a DebugInfo) -> EntityDebugInfo<'a>;
}

impl EntityDisplay for Entity {
    fn display_ref<'a>(&self, entity_ref: EntityRef<'a>) -> EntityRefDebugInfo<'a> {
        EntityRefDebugInfo {
            entity: *self,
            entity_ref,
        }
    }

    fn display<'a>(&self, info: &'a DebugInfo) -> EntityDebugInfo<'a> {
        info.for_entity(*self)
    }
}

pub trait EntityRefDisplay<'a> {
    fn display(&self, entity: Entity) -> EntityRefDebugInfo<'a>;
}

impl<'a> EntityRefDisplay<'a> for EntityRef<'a> {
    fn display(&self, entity: Entity) -> EntityRefDebugInfo<'a> {
        EntityRefDebugInfo {
            entity,
            entity_ref: *self,
        }
    }
}
