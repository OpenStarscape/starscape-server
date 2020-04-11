use std::error::Error;

use super::{ObjectId, Value};
use crate::EntityKey;

pub trait Protocol {
    fn serialize_property_update(
        &self,
        object: ObjectId,
        property: &str,
        value: &Value,
    ) -> Result<Vec<u8>, Box<Error>>;
}
