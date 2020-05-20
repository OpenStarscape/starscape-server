use super::*;
use crate::state::{EntityKey, State};

pub trait Connection {
    fn property_changed(
        &self,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>>;
    fn entity_destroyed(&self, state: &State, entity: EntityKey);
    fn object_to_entity(&self, object: ObjectId) -> Option<EntityKey>;
}

/// Used when we need a slotmap key before creating a connection
impl Connection for () {
    fn property_changed(&self, _: EntityKey, _: &str, _: &Encodable) -> Result<(), Box<dyn Error>> {
        panic!("property_changed() called on stub connection");
    }
    fn entity_destroyed(&self, _: &State, _: EntityKey) {
        panic!("entity_destroyed() called on stub connection");
    }
    fn object_to_entity(&self, _: ObjectId) -> Option<EntityKey> {
        panic!("object_to_entity() called on stub connection");
    }
}
