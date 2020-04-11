use bimap::BiHashMap;

use crate::state::EntityKey;

pub type ObjectId = u64;

pub struct ObjectMap {
    map: BiHashMap<EntityKey, ObjectId>,
    next_id: ObjectId,
}

impl ObjectMap {
    pub fn new() -> Self {
        Self {
            map: BiHashMap::new(),
            next_id: 1,
        }
    }

    pub fn register_entity(&mut self, entity: EntityKey) -> ObjectId {
        let id = self.next_id;
        self.next_id += 1;
        self.map.insert(entity, id);
        id
    }

    pub fn remove_entity(&mut self, entity: EntityKey) {
        self.map.remove_by_left(&entity);
    }

    pub fn get_object(&self, entity: EntityKey) -> Option<ObjectId> {
        self.map.get_by_left(&entity).cloned()
    }

    pub fn get_entity(&self, object: ObjectId) -> Option<EntityKey> {
        self.map.get_by_right(&object).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_if_same_entity_is_registered_twice() {
        panic!("Test not implemented");
    }

    #[test]
    fn errors_if_entity_is_removed_without_being_added() {
        panic!("Test not implemented");
    }
}
