use super::*;

type ConduitBuilder =
    Box<dyn Fn(ConnectionKey) -> Result<Box<dyn Conduit<Encodable, Decoded>>, String>>;

/// Conceptual owner of the various components in the state that make up a single "thing"
pub struct Entity {
    self_key: EntityKey,
    components: AnyMap,
    component_cleanup: Vec<Box<dyn FnOnce(&mut State)>>,
    conduit_builders: HashMap<&'static str, ConduitBuilder>,
}

impl Entity {
    pub fn new(self_key: EntityKey) -> Self {
        Self {
            self_key,
            components: AnyMap::new(),
            component_cleanup: Vec::new(),
            conduit_builders: HashMap::new(),
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

    /// Registers a conduit as a property/signal/action, shows error and does nothing else if there
    /// is already a registered conduit with the same name
    pub fn register_conduit<F>(&mut self, name: &'static str, f: F)
    where
        F: Fn(ConnectionKey) -> Result<Box<dyn Conduit<Encodable, Decoded>>, String> + 'static,
    {
        use std::collections::hash_map::Entry;
        match self.conduit_builders.entry(name) {
            Entry::Vacant(entry) => {
                entry.insert(Box::new(f));
            }
            Entry::Occupied(_) => {
                error!(
                    "conduit {} added to {:?} multiple times",
                    name, self.self_key
                );
            }
        }
    }

    /// Get the property of the given name
    pub fn conduit(
        &self,
        connection: ConnectionKey,
        name: &str,
    ) -> Result<Box<dyn Conduit<Encodable, Decoded>>, String> {
        match self.conduit_builders.get(name) {
            Some(builder) => builder(connection),
            None => Err(format!("entity does not have member {:?}", name)),
        }
    }

    /// Remove all components of this entity from the state
    pub fn finalize(&mut self, state: &mut State) {
        for cleanup in self.component_cleanup.drain(..) {
            cleanup(state);
        }
        self.components.clear();
        // TODO: register disconnected from connections
    }
}

// TODO: test
