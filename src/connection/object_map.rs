use super::*;
use bimap::BiHashMap;

/// The ID a client uses to identify an object. Maps to an EntityKey.
pub type ObjectId = u64;

/// A two-directional mapping between EntityKeys and ObjectIds. There is an object map for each client.
/// The implementation hides any mutex locking, exposing an interface that does not require mutable
/// access.
pub trait ObjectMap: Send + Sync {
    /// Returns the corresponding object ID if the entity is known
    fn get_object(&self, entity: EntityKey) -> Option<ObjectId>;
    /// Returns the corrosponding object ID, or creates a new object ID associated with entity
    fn get_or_create_object(&self, entity: EntityKey) -> ObjectId;
    /// Returns the corresponding entity if the object ID is known
    fn get_entity(&self, object: ObjectId) -> Option<EntityKey>;
    /// Removes an entity/object ID pair from the map. Future calls to get_object() with entity
    /// returns None, and future calls to get_or_create_object() creates a new ID. IDs are not
    /// recycled.
    fn remove_entity(&self, entity: EntityKey) -> Option<ObjectId>;
    /// Just needs to return self, only required because Rust is stupid
    fn as_encode_ctx(&self) -> &dyn EncodeCtx;
    /// Just needs to return self, only required because Rust is stupid
    fn as_decode_ctx(&self) -> &dyn DecodeCtx;
}

impl<T: ObjectMap> EncodeCtx for T {
    fn object_for(&self, entity: EntityKey) -> ObjectId {
        self.get_or_create_object(entity)
    }
}

impl<T: ObjectMap> DecodeCtx for T {
    fn entity_for(&self, object: ObjectId) -> Result<EntityKey, String> {
        self.get_entity(object)
            .ok_or_else(|| format!("unknwon object #{}", object))
    }
}

/// A RwLock of this type is the normal ObjectMap implementation
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
                if entity.is_null() {
                    error!("ObjectMap::get_or_create_object() given null entity");
                }
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
        let map = ObjectMapImpl::new();
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
        let map = ObjectMapImpl::new();
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
        let map = ObjectMapImpl::new();
        let e = mock_keys(2);
        assert_eq!(map.get_object(e[0]), None);
        map.get_or_create_object(e[0]);
        assert_eq!(map.get_object(e[1]), None);
    }

    #[test]
    fn nonexistant_objects_return_null() {
        let map = ObjectMapImpl::new();
        let e = mock_keys(1);
        let o = 47;
        assert_eq!(map.get_entity(o), None);
        map.get_or_create_object(e[0]);
        assert_eq!(map.get_entity(o), None);
    }

    #[test]
    fn entity_can_be_removed() {
        let map = ObjectMapImpl::new();
        let e = mock_keys(3);
        map.get_or_create_object(e[0]);
        let o = map.get_or_create_object(e[1]);
        assert_eq!(map.remove_entity(e[2]), None);
        assert_eq!(map.remove_entity(e[1]), Some(o));
        assert_eq!(map.remove_entity(e[1]), None);
    }

    #[test]
    fn object_and_entity_null_after_removal() {
        let map = ObjectMapImpl::new();
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
        let map = ObjectMapImpl::new();
        let e = mock_keys(1);
        let o = map.get_or_create_object(e[0]);
        assert_eq!(map.get_or_create_object(e[0]), o);
        assert_eq!(map.get_object(e[0]), Some(o));
        assert_eq!(map.get_or_create_object(e[0]), o);
    }

    #[test]
    fn same_entity_given_new_id_after_being_removed() {
        let map = ObjectMapImpl::new();
        let e = mock_keys(1);
        let o = map.get_or_create_object(e[0]);
        assert_eq!(map.remove_entity(e[0]), Some(o));
        assert_ne!(map.get_or_create_object(e[0]), o);
    }
}
