use super::*;
use bimap::BiHashMap;

/// The ID a client uses to identify an object. Maps to an EntityKey.
pub type ObjectId = u64;

/// A two-directional mapping between EntityKeys and ObjectIds. There is an object map for each client.
/// The implementation hides any mutex locking, exposing an interface that does not require mutable
/// access.
pub trait ObjectMap: DecodeCtx + Send + Sync {
    /// Returns the corresponding object ID if the entity is known
    fn get_object(&self, entity: GenericId) -> Option<ObjectId>;
    /// Returns the corrosponding object ID, or creates a new object ID associated with entity
    fn get_or_create_object(
        &self,
        handler: &dyn RequestHandler,
        entity: GenericId,
    ) -> RequestResult<ObjectId>;
    /// Returns the corresponding entity if the object ID is known
    fn get_entity(&self, object: ObjectId) -> Option<GenericId>;
    /// Removes an entity/object ID pair from the map. Future calls to get_object() with entity
    /// returns None, and future calls to get_or_create_object() creates a new ID. IDs are not
    /// recycled.
    fn remove_entity(&self, handler: &dyn RequestHandler, entity: GenericId) -> Option<ObjectId>;
    /// Subscribes the connection to the given property/signal
    fn subscribe(
        &self,
        handler: &dyn RequestHandler,
        id: GenericId,
        member: &str,
    ) -> RequestResult<()>;
    fn unsubscribe(
        &self,
        handler: &dyn RequestHandler,
        id: GenericId,
        member: &str,
    ) -> RequestResult<()>;
    /// Unsubscribes from all subscriptions including entity destruction
    fn finalize(&self, handler: &dyn RequestHandler);
}

struct EncodeCtxImpl<'a> {
    map: &'a dyn ObjectMap,
    handler: &'a dyn RequestHandler,
}

impl<'a> EncodeCtx for EncodeCtxImpl<'a> {
    fn object_for(&self, entity: GenericId) -> RequestResult<ObjectId> {
        self.map.get_or_create_object(self.handler, entity)
    }
}

pub fn new_encode_ctx<'a>(
    map: &'a dyn ObjectMap,
    handler: &'a dyn RequestHandler,
) -> impl EncodeCtx + 'a {
    EncodeCtxImpl { map, handler }
}

/// A RwLock of this type is the normal ObjectMap implementation
pub struct ObjectMapImpl {
    connection: ConnectionKey,
    map: BiHashMap<GenericId, ObjectId>,
    subscription_map: HashMap<
        GenericId,
        (
            Box<dyn Subscription>,
            HashMap<String, Box<dyn Subscription>>,
        ),
    >,
    next_id: ObjectId,
}

impl ObjectMapImpl {
    pub fn new(connection: ConnectionKey) -> RwLock<Self> {
        RwLock::new(ObjectMapImpl {
            connection,
            map: BiHashMap::new(),
            subscription_map: HashMap::new(),
            next_id: 1,
        })
    }
}

impl DecodeCtx for RwLock<ObjectMapImpl> {
    fn entity_for(&self, object: ObjectId) -> RequestResult<GenericId> {
        self.get_entity(object).ok_or(BadObject(object))
    }
}

impl ObjectMap for RwLock<ObjectMapImpl> {
    fn get_object(&self, entity: GenericId) -> Option<ObjectId> {
        self.read()
            .expect("failed to lock object map")
            .map
            .get_by_left(&entity)
            .cloned()
    }

    fn get_or_create_object(
        &self,
        handler: &dyn RequestHandler,
        entity: GenericId,
    ) -> RequestResult<ObjectId> {
        let obj = {
            let read = self.read().expect("failed to lock object map");
            read.map.get_by_left(&entity).cloned()
        };
        match obj {
            Some(obj) => Ok(obj),
            None => {
                if entity.is_null() {
                    error!("ObjectMap::get_or_create_object() given null entity");
                }
                let mut write = self.write().expect("failed to lock object map");
                // Because unlocking a reader and locking a writer isn't atomic, we need to check
                // that the object hasn't been created in the gap
                match write.map.get_by_left(&entity) {
                    Some(obj) => Ok(*obj),
                    None => {
                        let connection = write.connection;
                        write.subscription_map.insert(
                            entity,
                            (handler.subscribe(connection, entity, None)?, HashMap::new()),
                        );
                        let id = write.next_id;
                        write.next_id += 1;
                        let overwitten = write.map.insert(entity, id);
                        if overwitten != bimap::Overwritten::Neither {
                            panic!("logic error: overwrite bimap value: {:?}", overwitten)
                        }
                        Ok(id)
                    }
                }
            }
        }
    }

    fn get_entity(&self, object: ObjectId) -> Option<GenericId> {
        self.read()
            .expect("failed to lock object map")
            .map
            .get_by_right(&object)
            .cloned()
    }

    fn remove_entity(&self, handler: &dyn RequestHandler, entity: GenericId) -> Option<ObjectId> {
        let mut locked = self.write().expect("failed to lock object map");
        if let Some((destroy_sub, other_subs)) = locked.subscription_map.remove(&entity) {
            destroy_sub
                .finalize(handler)
                .or_log_warn("finalizing entity destruction subscription");
            for (_name, sub) in other_subs {
                sub.finalize(handler).or_log_warn("finalizing subscription");
            }
        }
        locked.map.remove_by_left(&entity).map(|(_, o)| o)
    }

    fn subscribe(
        &self,
        handler: &dyn RequestHandler,
        id: GenericId,
        member: &str,
    ) -> RequestResult<()> {
        use std::collections::hash_map::Entry;
        let mut locked = self.write().expect("failed to lock object map");
        let connection = locked.connection;
        let member_map = &mut locked.subscription_map.get_mut(&id).ok_or(BadId(id))?.1;
        match member_map.entry(member.to_string()) {
            Entry::Occupied(_) => return Err(BadRequest("already subscribed".into())),
            Entry::Vacant(entry) => {
                let sub = handler.subscribe(connection, id, Some(member))?;
                entry.insert(sub);
                Ok(())
            }
        }
    }

    fn unsubscribe(
        &self,
        handler: &dyn RequestHandler,
        id: GenericId,
        member: &str,
    ) -> RequestResult<()> {
        let mut locked = self.write().expect("failed to lock object map");
        let member_map = &mut locked.subscription_map.get_mut(&id).ok_or(BadId(id))?.1;
        match member_map.remove(member) {
            Some(sub) => sub.finalize(handler),
            None => Err(BadRequest("not subscribed".into())),
        }
    }

    fn finalize(&self, handler: &dyn RequestHandler) {
        let mut locked = self.write().expect("failed to lock object map");
        for (_id, (destroy_sub, other_subs)) in locked.subscription_map.drain() {
            destroy_sub.finalize(handler).or_log_warn(
                "finalizing entity destruction subscription during connection finalization",
            );
            for (_name, sub) in other_subs {
                sub.finalize(handler)
                    .or_log_warn("finalizing subscription during connection finalization");
            }
        }
    }
}

#[cfg(test)]
mod objects_tests {
    use super::*;

    fn new_object_map_impl() -> RwLock<ObjectMapImpl> {
        let c = mock_keys(1);
        ObjectMapImpl::new(c[0])
    }

    #[test]
    fn objects_can_be_created_and_looked_up() {
        let map = new_object_map_impl();
        let handler = MockRequestHandler::new(Ok(()));
        let e = mock_generic_ids(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|id| map.get_or_create_object(&handler, *id).unwrap())
            .collect();
        assert_eq!(map.get_entity(o[0]), Some(e[0]));
        assert_eq!(map.get_object(e[0]), Some(o[0]));
        assert_eq!(map.get_object(e[1]), Some(o[1]));
        assert_eq!(map.get_entity(o[1]), Some(e[1]));
    }

    #[test]
    fn object_ids_count_up_from_1() {
        let map = new_object_map_impl();
        let handler = MockRequestHandler::new(Ok(()));
        let e = mock_generic_ids(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|entity| map.get_or_create_object(&handler, *entity).unwrap())
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
        let map = new_object_map_impl();
        let handler = MockRequestHandler::new(Ok(()));
        let e = mock_generic_ids(2);
        assert_eq!(map.get_object(e[0]), None);
        map.get_or_create_object(&handler, e[0]).unwrap();
        assert_eq!(map.get_object(e[1]), None);
    }

    #[test]
    fn nonexistant_objects_return_null() {
        let map = new_object_map_impl();
        let handler = MockRequestHandler::new(Ok(()));
        let e = mock_generic_ids(1);
        let o = 47;
        assert_eq!(map.get_entity(o), None);
        map.get_or_create_object(&handler, e[0]).unwrap();
        assert_eq!(map.get_entity(o), None);
    }

    #[test]
    fn entity_can_be_removed() {
        let map = new_object_map_impl();
        let handler = MockRequestHandler::new(Ok(()));
        let e = mock_generic_ids(3);
        map.get_or_create_object(&handler, e[0]).unwrap();
        let o = map.get_or_create_object(&handler, e[1]).unwrap();
        assert_eq!(map.remove_entity(&handler, e[2]), None);
        assert_eq!(map.remove_entity(&handler, e[1]), Some(o));
        assert_eq!(map.remove_entity(&handler, e[1]), None);
    }

    #[test]
    fn object_and_entity_null_after_removal() {
        let map = new_object_map_impl();
        let handler = MockRequestHandler::new(Ok(()));
        let e = mock_generic_ids(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|entity| map.get_or_create_object(&handler, *entity).unwrap())
            .collect();
        map.remove_entity(&handler, e[1]);
        assert_eq!(map.get_entity(o[1]), None);
        assert_eq!(map.get_object(e[1]), None);
    }

    #[test]
    fn get_or_create_object_is_idempotent() {
        let map = new_object_map_impl();
        let handler = MockRequestHandler::new(Ok(()));
        let e = mock_generic_ids(1);
        let o = map.get_or_create_object(&handler, e[0]).unwrap();
        assert_eq!(map.get_or_create_object(&handler, e[0]).unwrap(), o);
        assert_eq!(map.get_object(e[0]), Some(o));
        assert_eq!(map.get_or_create_object(&handler, e[0]).unwrap(), o);
    }

    #[test]
    fn same_entity_given_new_id_after_being_removed() {
        let map = new_object_map_impl();
        let handler = MockRequestHandler::new(Ok(()));
        let e = mock_generic_ids(1);
        let o = map.get_or_create_object(&handler, e[0]).unwrap();
        assert_eq!(map.remove_entity(&handler, e[0]), Some(o));
        assert_ne!(map.get_or_create_object(&handler, e[0]).unwrap(), o);
    }
}
