use crate::state::{ConduitKey, State};

/// Conceptual owner of the various components in the state that make up a single "thing"
pub trait Entity {
    /// Attaches a conduit to the entity
    fn add_conduit(&mut self, name: &'static str, conduit: ConduitKey);
    /// Get the conduit for the given property name
    fn conduit(&self, property: &str) -> Result<ConduitKey, String>;
    /// Remove all components of this entity from the state
    fn destroy(&mut self, state: &mut State);
}
