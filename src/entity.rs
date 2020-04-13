use crate::state::{PropertyKey, State};

/// Conceptual owner of the various components in the state that make up a single "thing"
pub trait Entity {
    /// Attaches a conduit to the entity
    fn add_property(&mut self, name: &'static str, key: PropertyKey);
    /// Get the conduit for the given property name
    fn property(&self, name: &str) -> Result<PropertyKey, String>;
    /// Remove all components of this entity from the state
    fn destroy(&mut self, state: &mut State);
}
