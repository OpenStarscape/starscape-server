use super::*;

pub struct EntityStoreImpl {
    entities: DenseSlotMap<EntityKey, Entity>,
}

impl EntityStoreImpl {
    pub fn new() -> Self {
        Self {
            entities: DenseSlotMap::with_key(),
        }
    }
}

impl EntityStore for EntityStoreImpl {
    fn register_property(
        &mut self,
        entity_key: EntityKey,
        name: &'static str,
        conduit: Box<dyn Conduit>,
    ) {
        if let Some(entity) = self.entities.get_mut(entity_key) {
            let property = PropertyImpl::new(entity_key, name, conduit);
            entity.register_property(name, Arc::new(property));
        } else {
            eprintln!("Failed to register proprty on entity {:?}", entity_key);
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

    fn get_property(
        &self,
        entity_key: EntityKey,
        name: &str,
    ) -> Result<&Arc<dyn Property>, String> {
        let entity = self
            .entities
            .get(entity_key)
            .ok_or(format!("bad entity {:?}", entity_key))?;
        let property = entity
            .get_property(name)
            .ok_or(format!("entity does not have property {:?}", name))?;
        Ok(property)
    }
}
