use super::*;
use bimap::BiHashMap;

pub struct ObjectMapImpl {
    map: BiHashMap<EntityKey, ObjectId>,
    next_id: ObjectId,
}

impl ObjectMapImpl {
    pub fn new() -> RwLock<Self> {
        RwLock::new(ObjectMapImpl {
            map: BiHashMap::new(),
            next_id: 1,
        })
    }
}

impl ObjectMap for RwLock<ObjectMapImpl> {
    fn get_object(&self, entity: EntityKey) -> Option<ObjectId> {
        self.read()
            .expect("failed to lock object map")
            .map
            .get_by_left(&entity)
            .cloned()
    }

    fn get_or_create_object(&self, entity: EntityKey) -> ObjectId {
        let obj = {
            let read = self.read().expect("failed to lock object map");
            read.map.get_by_left(&entity).cloned()
        };
        match obj {
            Some(obj) => obj,
            None => {
                let mut write = self.write().expect("failed to lock object map");
                // Because unlocking a reader and locking a writer isn't atomic, we need to check
                // that the object hasn't been created in the gap
                match write.map.get_by_left(&entity) {
                    Some(obj) => *obj,
                    None => {
                        let id = write.next_id;
                        write.next_id += 1;
                        let overwitten = write.map.insert(entity, id);
                        if overwitten != bimap::Overwritten::Neither {
                            panic!("logic error: overwrite bimap value: {:?}", overwitten)
                        }
                        id
                    }
                }
            }
        }
    }

    fn get_entity(&self, object: ObjectId) -> Option<EntityKey> {
        self.read()
            .expect("failed to lock object map")
            .map
            .get_by_right(&object)
            .cloned()
    }

    fn remove_entity(&self, entity: EntityKey) -> Option<ObjectId> {
        self.write()
            .expect("failed to lock object map")
            .map
            .remove_by_left(&entity)
            .map(|(_, o)| o)
    }

    fn as_encode_ctx(&self) -> &dyn EncodeCtx {
        self
    }

    fn as_decode_ctx(&self) -> &dyn DecodeCtx {
        self
    }
}

#[cfg(test)]
mod objects_tests {
    use super::*;

    #[test]
    fn objects_can_be_created_and_looked_up() {
        let mut map = ObjectMapImpl::new();
        let e = mock_keys(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|entity| map.get_or_create_object(*entity))
            .collect();
        assert_eq!(map.get_entity(o[0]), Some(e[0]));
        assert_eq!(map.get_object(e[0]), Some(o[0]));
        assert_eq!(map.get_object(e[1]), Some(o[1]));
        assert_eq!(map.get_entity(o[1]), Some(e[1]));
    }

    #[test]
    fn object_ids_count_up_from_1() {
        let mut map = ObjectMapImpl::new();
        let e = mock_keys(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|entity| map.get_or_create_object(*entity))
            .collect();
        assert_eq!(o[0], 1);
        assert_eq!(o[1], 2);
        assert!(map.get_entity(0).is_none());
        assert!(map.get_entity(1).is_some());
        assert!(map.get_entity(2).is_some());
        assert!(map.get_entity(3).is_none());
    }

    #[test]
    fn nonexistant_entities_return_null() {
        let mut map = ObjectMapImpl::new();
        let e = mock_keys(2);
        assert_eq!(map.get_object(e[0]), None);
        map.get_or_create_object(e[0]);
        assert_eq!(map.get_object(e[1]), None);
    }

    #[test]
    fn nonexistant_objects_return_null() {
        let mut map = ObjectMapImpl::new();
        let e = mock_keys(1);
        let o = 47;
        assert_eq!(map.get_entity(o), None);
        map.get_or_create_object(e[0]);
        assert_eq!(map.get_entity(o), None);
    }

    #[test]
    fn entity_can_be_removed() {
        let mut map = ObjectMapImpl::new();
        let e = mock_keys(3);
        map.get_or_create_object(e[0]);
        let o = map.get_or_create_object(e[1]);
        assert_eq!(map.remove_entity(e[2]), None);
        assert_eq!(map.remove_entity(e[1]), Some(o));
        assert_eq!(map.remove_entity(e[1]), None);
    }

    #[test]
    fn object_and_entity_null_after_removal() {
        let mut map = ObjectMapImpl::new();
        let e = mock_keys(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|entity| map.get_or_create_object(*entity))
            .collect();
        map.remove_entity(e[1]);
        assert_eq!(map.get_entity(o[1]), None);
        assert_eq!(map.get_object(e[1]), None);
    }

    #[test]
    fn get_or_create_object_is_idempotent() {
        let mut map = ObjectMapImpl::new();
        let e = mock_keys(1);
        let o = map.get_or_create_object(e[0]);
        assert_eq!(map.get_or_create_object(e[0]), o);
        assert_eq!(map.get_object(e[0]), Some(o));
        assert_eq!(map.get_or_create_object(e[0]), o);
    }

    #[test]
    fn same_entity_given_new_id_after_being_removed() {
        let mut map = ObjectMapImpl::new();
        let e = mock_keys(1);
        let o = map.get_or_create_object(e[0]);
        assert_eq!(map.remove_entity(e[0]), Some(o));
        assert_ne!(map.get_or_create_object(e[0]), o);
    }
}

/*
#[cfg(test)]
mod resolve_tests {
    use super::Encodable::*;
    use super::*;

    fn map_entities_objects() -> (ObjectMap, Vec<EntityKey>, Vec<ObjectId>) {
        let mut map = ObjectMap::new();
        let e = mock_keys(3);
        let o = e
            .iter()
            .map(|entity| map.register_entity(*entity))
            .collect();
        (map, e, o)
    }

    #[test]
    fn resolves_scaler_without_changing() {
        let (mut map, _, _) = map_entities_objects();
        assert_eq!(map.resolve(&Scaler(12.5)), None);
    }

    #[test]
    fn resolves_list_without_changing() {
        let (mut map, _, _) = map_entities_objects();
        assert_eq!(map.resolve(&List(vec![Integer(7), Integer(12)])), None);
    }

    #[test]
    fn resolves_entity_to_object_id() {
        let (mut map, e, o) = map_entities_objects();
        assert_eq!(map.resolve(&Entity(e[0])), Some(Integer(o[0] as i64)));
        assert_eq!(map.resolve(&Entity(e[1])), Some(Integer(o[1] as i64)));
    }

    #[test]
    fn resolves_list_of_entities_to_object_ids() {
        let (mut map, e, o) = map_entities_objects();
        let original = List(e.iter().map(|e| Entity(*e)).collect());
        let resolved = List(o.iter().map(|o| Integer(*o as i64)).collect());
        assert_eq!(map.resolve(&original), Some(resolved));
    }

    #[test]
    fn resolves_list_with_entity_and_other_stuff() {
        let (mut map, e, o) = map_entities_objects();
        let original = List(vec![Integer(42), Entity(e[0]), Null]);
        let resolved = List(vec![Integer(42), Integer(o[0] as i64), Null]);
        assert_eq!(map.resolve(&original), Some(resolved));
    }

    #[test]
    fn resolves_nested_lists_with_entities() {
        let (mut map, e, o) = map_entities_objects();
        let original = List(vec![
            List(vec![List(vec![Entity(e[1]), Entity(e[2]), Null])]),
            Integer(42),
            Entity(e[0]),
        ]);
        let resolved = List(vec![
            List(vec![List(vec![
                Integer(o[1] as i64),
                Integer(o[2] as i64),
                Null,
            ])]),
            Integer(42),
            Integer(o[0] as i64),
        ]);
        assert_eq!(map.resolve(&original), Some(resolved));
    }
}
*/
