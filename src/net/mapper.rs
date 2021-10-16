use std::collections::hash_map::{Entry, HashMap};

use hecs::{Entity, QueryOneError, World};

use super::NetId;

#[cfg(feature = "server")]
use super::IdGen;

pub struct EntityMapper {
    entity_by_id: HashMap<NetId, Entity>,
}

impl EntityMapper {
    #[inline(always)]
    pub fn new() -> Self {
        EntityMapper {
            entity_by_id: HashMap::new(),
        }
    }

    #[inline(always)]
    pub fn get(&self, nid: NetId) -> Option<Entity> {
        self.entity_by_id.get(&nid).copied()
    }

    #[cfg(feature = "client")]
    #[inline]
    pub fn get_or_spawn(&mut self, world: &mut World, nid: NetId) -> Entity {
        match self.entity_by_id.entry(nid) {
            Entry::Occupied(mut entry) => {
                let entity = *entry.get();

                match world.query_one_mut::<&NetId>(entity) {
                    Ok(id) => {
                        assert_eq!(*id, nid, "NetId modified on entity");
                    }
                    Err(QueryOneError::Unsatisfied) => {
                        panic!("NetId component was removed on entity");
                    }
                    Err(QueryOneError::NoSuchEntity) => {
                        let entity = world.spawn((nid,));
                        entry.insert(entity);
                    }
                }

                entity
            }
            Entry::Vacant(entry) => {
                let entity = world.spawn((nid,));
                entry.insert(entity);
                entity
            }
        }
    }

    #[cfg(feature = "server")]
    #[inline(always)]
    pub(super) fn new_nid(&mut self, gen: &mut IdGen, entity: Entity) -> NetId {
        let nid = gen.gen_nid();
        let old = self.entity_by_id.insert(nid, entity);
        debug_assert!(old.is_none(), "Non-unique NetId mapped");
        nid
    }

    #[cfg(feature = "server")]
    #[inline(always)]
    pub(super) fn iter_removed<'a>(&'a self, world: &'a World) -> impl Iterator<Item = NetId> + 'a {
        self.entity_by_id
            .iter()
            .filter_map(move |(nid, e)| (!world.contains(*e)).then(|| *nid))
    }

    #[cfg(feature = "server")]
    #[inline(always)]
    pub(super) fn clear_removed<'a>(&'a mut self, world: &'a World) {
        self.entity_by_id.retain(|_, e| world.contains(*e))
    }
}
