use std::collections::HashMap;

use super::*;

new_key_type! {
    pub struct EntityKey;
}

#[derive(Debug)]
enum ComponentKey {
    Body(BodyKey),
    Ship(ShipKey),
}

/// Conceptual owner of the various components in the state that make up a single "thing"
pub struct Entity {
    components: Vec<ComponentKey>,
    properties: HashMap<&'static str, Box<dyn Property>>,
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            components: Vec::new(),
            properties: HashMap::new(),
        }
    }
}

impl Entity {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_property(&mut self, name: &'static str, property: Box<dyn Property>) {
        if self.properties.insert(name, property).is_some() {
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
    pub fn get_property(&self, name: &str) -> Option<&dyn Property> {
        self.properties.get(name).map(|prop| &**prop)
    }

    /// Remove all components of this entity from the state
    pub fn finalize(&mut self, state: &mut State) {
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
        for (_name, prop) in self.properties.drain() {
            prop.finalize(state);
        }
        // TODO: register disconnected from connections
    }
}
