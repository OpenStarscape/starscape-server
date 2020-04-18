use std::collections::HashMap;

use crate::state::{BodyKey, PropertyKey, ShipKey, State};

#[derive(Debug)]
enum ComponentKey {
    Body(BodyKey),
    Ship(ShipKey),
}

/// Conceptual owner of the various components in the state that make up a single "thing"
pub struct Entity {
    components: Vec<ComponentKey>,
    properties: HashMap<&'static str, PropertyKey>,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            properties: HashMap::new(),
        }
    }

    /// Called by the state
    pub fn register_property(&mut self, name: &'static str, key: PropertyKey) {
        if self.properties.insert(name, key).is_some() {
            eprintln!(
                "entity already has property {}, replacing with new one",
                name
            );
        }
    }

    pub fn register_body(&mut self, body: BodyKey) {
        self.components.push(ComponentKey::Body(body));
    }

    pub fn register_ship(&mut self, ship: ShipKey) {
        self.components.push(ComponentKey::Ship(ship));
    }

    /// Get the property of the given name
    pub fn property(&self, name: &str) -> Option<PropertyKey> {
        self.properties.get(name).cloned()
    }

    /// Remove all components of this entity from the state
    pub fn destroy(&mut self, state: &mut State) {
        for component in &self.components {
            if match component {
                ComponentKey::Body(body) => state.remove_body(*body).is_err(),
                ComponentKey::Ship(ship) => state.ships.remove(*ship).is_none(),
            } {
                eprintln!(
                    "component {:?} part of entity being destroyed, but is not in state",
                    component
                );
            }
        }
        self.components.clear();
        for property in self.properties.values() {
            if state.properties.remove(*property).is_none() {
                eprintln!(
                    "property {:?} part of entity being destoryed, but is not in state",
                    property
                );
            }
        }
        self.properties.clear()
        // TODO: register disconnected from connections
    }
}
