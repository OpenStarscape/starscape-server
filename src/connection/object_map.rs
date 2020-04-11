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
        if let Err(_) = self.map.insert_no_overwrite(entity, id) {
            panic!("{:?} already in the bimap", entity);
        }
        id
    }

    pub fn remove_entity(&mut self, entity: EntityKey) -> Option<ObjectId> {
        self.map.remove_by_left(&entity).map(|(e, o)| o)
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

    /// Should only be used once per test
    fn mock_keys<T: slotmap::Key>(number: u32) -> Vec<T> {
        let mut map = slotmap::DenseSlotMap::with_key();
        (0..number).map(|_| map.insert(())).collect()
    }

    #[test]
    fn registered_entities_and_objects_can_be_looked_up() {
        let mut map = ObjectMap::new();
        let e = mock_keys(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|entity| map.register_entity(*entity))
            .collect();
        assert_eq!(map.get_entity(o[0]), Some(e[0]));
        assert_eq!(map.get_object(e[0]), Some(o[0]));
        assert_eq!(map.get_object(e[1]), Some(o[1]));
        assert_eq!(map.get_entity(o[1]), Some(e[1]));
    }

    #[test]
    fn nonexistant_entities_and_objects_return_null() {
        let mut map = ObjectMap::new();
        let e = mock_keys(2);
        let o = 47;
        assert_eq!(map.get_entity(o), None);
        assert_eq!(map.get_object(e[0]), None);
        map.register_entity(e[0]);
        assert_eq!(map.get_entity(o), None);
        assert_eq!(map.get_object(e[1]), None);
    }

    #[test]
    fn correct_object_removed() {
        let mut map = ObjectMap::new();
        let e = mock_keys(3);
        map.register_entity(e[0]);
        let o = map.register_entity(e[1]);
        assert_eq!(map.remove_entity(e[2]), None);
        assert_eq!(map.remove_entity(e[1]), Some(o));
        assert_eq!(map.remove_entity(e[1]), None);
    }

    #[test]
    fn object_and_entity_null_after_removal() {
        let mut map = ObjectMap::new();
        let e = mock_keys(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|entity| map.register_entity(*entity))
            .collect();
        map.remove_entity(e[1]);
        assert_eq!(map.get_entity(o[1]), None);
        assert_eq!(map.get_object(e[1]), None);
    }

    #[test]
    #[should_panic]
    fn panics_if_same_entity_is_registered_twice() {
        let mut map = ObjectMap::new();
        let e = mock_keys(1);
        map.register_entity(e[0]);
        map.register_entity(e[0]);
    }
}
