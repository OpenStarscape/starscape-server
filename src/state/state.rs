use super::*;

new_key_type! {
    pub struct BodyKey;
    pub struct ShipKey;
}

pub type PendingNotifications = RwLock<Vec<Weak<dyn Subscriber>>>;

type ComponentMap<T> = DenseSlotMap<ComponentKey<T>, (EntityKey, T)>;
type ComponentElement<T> = (PhantomData<T>, Element<()>);

/// The entire game state at a single point in time
pub struct State {
    /// Current time in seconds since the start of the game
    pub time: f64,
    entities: DenseSlotMap<EntityKey, Entity>,
    components: AnyMap,
    component_list_elements: Mutex<AnyMap>, // TODO: change to subscription trackers
    pub pending_updates: PendingNotifications,
}

impl Default for State {
    fn default() -> Self {
        Self {
            time: 0.0,
            entities: DenseSlotMap::with_key(),
            components: AnyMap::new(),
            component_list_elements: Mutex::new(AnyMap::new()),
            pending_updates: RwLock::new(Vec::new()),
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the key for the newly created entity
    pub fn create_entity(&mut self) -> EntityKey {
        self.entities.insert_with_key(Entity::new)
    }

    /// Removes the given entity and all its components from the state
    pub fn destroy_entity(&mut self, entity: EntityKey) -> Result<(), Box<dyn Error>> {
        let mut entity = self
            .entities
            .remove(entity)
            .ok_or_else(|| format!("destroy_entity() called on invalid entity {:?}", entity))?;
        entity.finalize(self);
        Ok(())
    }

    /// Attaches the new component to the given entity
    /// Panics if the entity already has a component of the given type
    pub fn install_component<T: 'static>(&mut self, entity: EntityKey, component: T) {
        let e = self
            .entities
            .get_mut(entity)
            .expect("can not add component to invalid entity");
        let map: &mut ComponentMap<T> = self
            .components
            .entry()
            .or_insert_with(DenseSlotMap::with_key);
        let key = map.insert((entity, component));
        e.register_component(key, move |state| state.remove_component(key));
        self.trigger_component_list_element_update::<T>();
        // TODO: test that an update is sent to the component list element
    }

    /// Returns the component of type T attached to the given entity
    /// or None if no such component is found
    pub fn component<T: 'static>(&self, entity: EntityKey) -> Result<&T, String> {
        let e = self
            .entities
            .get(entity)
            .ok_or_else(|| format!("failed to get component on invalid entity {:?}", entity))?;
        let component = *e.component_key().ok_or_else(|| {
            format!(
                "failed to get invalid component {} on entity {:?}",
                type_name::<T>(),
                entity
            )
        })?;
        let map: &ComponentMap<T> = self
            .components
            .get()
            .ok_or_else(|| format!("no components of type {}", type_name::<T>()))?;
        match map.get(component) {
            Some(v) => Ok(&v.1),
            None => Err(format!(
                "invalid component {} ID {:?}",
                type_name::<T>(),
                component
            )),
        }
    }

    /// Returns a mutable reference to the given component
    /// or None if no such component is found
    pub fn component_mut<T: 'static>(
        &mut self,
        entity: EntityKey,
    ) -> Result<(&PendingNotifications, &mut T), String> {
        let e = self
            .entities
            .get(entity)
            .ok_or_else(|| format!("failed to get component on invalid entity {:?}", entity))?;
        let component = *e.component_key().ok_or_else(|| {
            format!(
                "failed to get invalid component {} on entity {:?}",
                type_name::<T>(),
                entity
            )
        })?;
        let map: &mut ComponentMap<T> = self
            .components
            .get_mut()
            .ok_or_else(|| format!("no components of type {}", type_name::<T>()))?;
        match map.get_mut(component) {
            Some(v) => Ok((&self.pending_updates, &mut v.1)),
            None => Err(format!(
                "invalid component {} ID {:?}",
                type_name::<T>(),
                component
            )),
        }
    }

    /// Returns an iterator over all components of a particular type
    pub fn components_iter<'a, T: 'static>(
        &'a self,
    ) -> Box<dyn std::iter::Iterator<Item = (EntityKey, &T)> + 'a> {
        if let Some(map) = self.components.get::<ComponentMap<T>>() {
            Box::new(map.values().map(|(entity, value)| (*entity, value)))
        } else {
            Box::new(std::iter::empty())
        }
    }

    /// Returns a mutable iterator over all components of a particular type
    pub fn components_iter_mut<'a, T: 'static>(
        &'a mut self,
    ) -> (
        &PendingNotifications,
        Box<dyn std::iter::Iterator<Item = (EntityKey, &mut T)> + 'a>,
    ) {
        (
            &self.pending_updates,
            match self.components.get_mut::<ComponentMap<T>>() {
                Some(map) => Box::new(map.values_mut().map(|(entity, value)| (*entity, value))),
                None => Box::new(std::iter::empty()),
            },
        )
    }

    /// Subscribe to be notified when a component of type T is created or destroyed
    pub fn subscribe_to_component_list<T: 'static>(
        &self,
        subscriber: &Arc<dyn Subscriber>,
    ) -> Result<(), Box<dyn Error>> {
        let mut map = self
            .component_list_elements
            .lock()
            .expect("failed to lock component elements");
        let element = &map
            .entry::<ComponentElement<T>>()
            .or_insert_with(|| (PhantomData, Element::new(())))
            .1;
        element.subscribe(subscriber)
    }

    pub fn unsubscribe_from_component_list<T: 'static>(
        &self,
        subscriber: &Weak<dyn Subscriber>,
    ) -> Result<(), Box<dyn Error>> {
        let mut map = self
            .component_list_elements
            .lock()
            .expect("failed to lock component elements");
        let element = &map
            .entry::<ComponentElement<T>>()
            .or_insert_with(|| (PhantomData, Element::new(())))
            .1;
        element.unsubscribe(subscriber)
    }

    /// Create a property for an entity
    /// Panics if entity doesn't exist or already has a property with this name
    pub fn install_property(
        &mut self,
        entity_key: EntityKey,
        name: &'static str,
        conduit: Box<dyn Conduit>,
    ) {
        if let Some(entity) = self.entities.get_mut(entity_key) {
            let property = PropertyImpl::new(entity_key, name, conduit);
            entity.register_property(name, Arc::new(property));
        } else {
            panic!("Failed to register proprty on entity {:?}", entity_key);
        }
    }

    /// Returns the property with the given name on the entity
    /// (properties are for clients, generally not direct engine use)
    pub fn property(
        &self,
        entity_key: EntityKey,
        name: &str,
    ) -> Result<&Arc<dyn Property>, String> {
        let entity = self
            .entities
            .get(entity_key)
            .ok_or(format!("bad entity {:?}", entity_key))?;
        let property = entity
            .property(name)
            .ok_or(format!("entity does not have property {:?}", name))?;
        Ok(property)
    }

    fn remove_component<T: 'static>(&mut self, component: ComponentKey<T>) {
        let mut remove_map = false;
        let mut update_component_list_element = false;
        match self.components.get_mut::<ComponentMap<T>>() {
            Some(map) => {
                if map.remove(component).is_some() {
                    update_component_list_element = true;
                } else {
                    eprintln!("failed to remove {} {:?}", type_name::<T>(), component);
                }
                remove_map = map.is_empty();
            }
            None => {
                eprintln!("no components of type {} to remove", type_name::<T>());
            }
        }

        if remove_map {
            self.components.remove::<ComponentMap<T>>();
        }

        if update_component_list_element {
            self.trigger_component_list_element_update::<T>();
        }
    }

    fn trigger_component_list_element_update<T: 'static>(&mut self) {
        let mut map = self
            .component_list_elements
            .lock()
            .expect("failed to lock component elements");
        let element = &mut map
            .entry::<ComponentElement<T>>()
            .or_insert_with(|| (PhantomData, Element::new(())))
            .1;
        element.get_mut(&self.pending_updates);
    }

    #[cfg(test)]
    pub fn assert_is_empty(&self) {
        assert!(self.components.is_empty());
        assert!(self.entities.is_empty());
        // pending_updates intentionally not checked
    }
}

impl RequestHandler for State {
    fn set(&mut self, entity: EntityKey, property: &str, value: &Decodable) -> Result<(), String> {
        let property = self.property(entity, property)?.clone();
        property.set_value(self, value)
    }

    fn get(&self, entity: EntityKey, property: &str) -> Result<Encodable, String> {
        let property = self.property(entity, property)?;
        property.get_value(self)
    }

    fn subscribe(
        &mut self,
        entity: EntityKey,
        property: &str,
        connection: ConnectionKey,
    ) -> Result<(), String> {
        let property = self.property(entity, property)?;
        property.subscribe(self, connection)?;
        Ok(())
    }

    fn unsubscribe(
        &mut self,
        entity: EntityKey,
        property: &str,
        connection: ConnectionKey,
    ) -> Result<(), String> {
        let property = self.property(entity, property)?;
        property.unsubscribe(self, connection)?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn is_empty_by_default() {
        let state = State::new();
        state.assert_is_empty();
    }

    #[test]
    fn mock_keys_all_different() {
        let k: Vec<EntityKey> = mock_keys(3);
        assert_eq!(k.len(), 3);
        assert_ne!(k[0], k[1]);
        assert_ne!(k[0], k[2]);
        assert_ne!(k[1], k[2]);
    }
}
