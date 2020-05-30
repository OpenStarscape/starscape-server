use super::*;
use crate::state::*;

pub trait EntityStore {
    fn register_property(&mut self, entity: EntityKey, name: &'static str, key: PropertyKey);
    fn new_entity(&mut self) -> EntityKey;
    fn register_body(&mut self, entity: EntityKey, body: BodyKey);
    fn register_ship(&mut self, entity: EntityKey, ship: ShipKey);
    fn property(&self, entity: EntityKey, property: &str) -> Result<PropertyKey, String>;
}

impl dyn EntityStore {
    pub fn default_impl() -> Box<dyn EntityStore> {
        Box::new(EntityStoreImpl::new())
    }
}
