use std::error::Error;

use super::Value;
use crate::state::{EntityKey, State};

pub trait Connection {
    fn property_changed(
        &self,
        entity: EntityKey,
        property: &str,
        value: &Value,
    ) -> Result<(), Box<dyn Error>>;
    fn entity_destroyed(&self, state: &State, entity: EntityKey);
    fn subscribe_to(&self, state: &State, entity: EntityKey, property: &str);
}
