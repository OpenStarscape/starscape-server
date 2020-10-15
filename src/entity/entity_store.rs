use super::*;

pub trait EntityStore {
    fn create_entity(&mut self) -> EntityKey;
    fn register_body(&mut self, entity: EntityKey, body: BodyKey);
    fn register_ship(&mut self, entity: EntityKey, ship: ShipKey);
    fn create_property(&mut self, entity: EntityKey, name: &'static str, conduit: Box<dyn Conduit>);
    fn get_property(&self, entity: EntityKey, name: &str) -> Result<&Arc<dyn Property>, String>;
}

impl dyn EntityStore {
    pub fn default_impl() -> Box<dyn EntityStore> {
        Box::new(EntityStoreImpl::new())
    }
}
