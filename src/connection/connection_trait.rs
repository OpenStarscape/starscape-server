use std::error::Error;

use super::Value;
use crate::state::{EntityKey, State};

pub trait Connection {
    fn send_property_update(
        &self,
        entity: EntityKey,
        property: &str,
        value: &Value,
    ) -> Result<(), Box<Error>>;
    fn register_object(&self, entity: EntityKey);
    fn subscribe_to(&self, state: &State, entity: EntityKey, property: &str);
}
