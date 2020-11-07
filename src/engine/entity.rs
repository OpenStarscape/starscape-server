use super::*;

/// Conceptual owner of the various components in the state that make up a single "thing"
pub struct Entity {
    self_key: EntityKey,
    components: AnyMap,
    component_cleanup: Vec<Box<dyn FnOnce(&mut State)>>,
    properties: HashMap<&'static str, Arc<dyn Property>>,
}

impl Entity {
    pub fn new(self_key: EntityKey) -> Self {
        Self {
            self_key,
            components: AnyMap::new(),
            component_cleanup: Vec::new(),
            properties: HashMap::new(),
        }
    }

    /// Register that a component of type T is attached to this entity
    /// Panics if a component of type T is already registered
    pub fn register_component<T, F>(&mut self, component: ComponentKey<T>, cleanup: F)
    where
        T: 'static,
        F: FnOnce(&mut State) + 'static,
    {
        if self.components.insert(component).is_some() {
            panic!(
                "multiple {}s added to {:?}",
                type_name::<T>(),
                self.self_key
            )
        }
        self.component_cleanup.push(Box::new(cleanup));
    }

    pub fn component_key<T: 'static>(&self) -> Option<&ComponentKey<T>> {
        self.components.get::<ComponentKey<T>>()
    }

    /// Registers the given property
    /// panics if a property with the same name is already registered
    pub fn register_property(&mut self, name: &'static str, property: Arc<dyn Property>) {
        if self.properties.insert(name, property).is_some() {
            panic!(
                "property \"{}\" added to {:?} multiple times",
                name, self.self_key
            )
        }
    }

    /// Get the property of the given name
    pub fn property(&self, name: &str) -> Option<&Arc<dyn Property>> {
        self.properties.get(name)
    }

    /// Remove all components of this entity from the state
    pub fn finalize(&mut self, state: &mut State) {
        for cleanup in self.component_cleanup.drain(..) {
            cleanup(state);
        }
        self.components.clear();
        for (_name, prop) in self.properties.drain() {
            prop.finalize(state);
        }
        // TODO: register disconnected from connections
    }
}

// TODO: test
