use super::*;

/// The ID a client uses to identify an object. Maps to an EntityKey.
pub type ObjectId = u64;

/// A two-directional mapping of entity keys to object IDs. There is an object map for each client.
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
    fn entity_for(&self, object: ObjectId) -> Result<EntityKey, Box<dyn Error>> {
        self.get_entity(object)
            .ok_or_else(|| format!("invalid object {}", object).into())
    }
}
