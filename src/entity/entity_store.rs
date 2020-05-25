use slotmap::DenseSlotMap;

use super::*;
use crate::state::*;

new_key_type! {
    pub struct EntityKey;
}

pub trait EntityStore {
    fn register_property(&mut self, entity: EntityKey, name: &'static str, key: PropertyKey);
    fn new_entity(&mut self) -> EntityKey;
    fn register_body(&mut self, entity: EntityKey, body: BodyKey);
    fn register_ship(&mut self, entity: EntityKey, ship: ShipKey);
    fn property(&self, entity: EntityKey, property: &str) -> Result<PropertyKey, String>;
}

impl dyn EntityStore {
    pub fn default_impl() -> Box<dyn EntityStore> {
        Box::new(EntityStoreImpl {
            entities: DenseSlotMap::with_key(),
        })
    }
}

struct EntityStoreImpl {
    pub entities: DenseSlotMap<EntityKey, Entity>,
}

impl EntityStore for EntityStoreImpl {
    fn register_property(&mut self, entity: EntityKey, name: &'static str, key: PropertyKey) {
        if let Some(entity) = self.entities.get_mut(entity) {
            entity.register_property(name, key);
        } else {
            eprintln!("Failed to register proprty on entity {:?}", entity);
        }
    }

    fn new_entity(&mut self) -> EntityKey {
        self.entities.insert(Entity::new())
    }

    fn register_body(&mut self, entity: EntityKey, body: BodyKey) {
        if let Some(entity) = self.entities.get_mut(entity) {
            entity.register_body(body);
        } else {
            eprintln!("Failed to register proprty on entity {:?}", entity);
        }
    }

    fn register_ship(&mut self, entity: EntityKey, ship: ShipKey) {
        if let Some(entity) = self.entities.get_mut(entity) {
            entity.register_ship(ship);
        } else {
            eprintln!("Failed to register proprty on entity {:?}", entity);
        }
    }

    fn property(&self, entity_key: EntityKey, property: &str) -> Result<PropertyKey, String> {
        let entity = self
            .entities
            .get(entity_key)
            .ok_or(format!("bad entity {:?}", entity_key))?;
        let property_key = entity
            .property(property)
            .ok_or(format!("entity does not have property {:?}", property))?;
        Ok(property_key)
    }
}
