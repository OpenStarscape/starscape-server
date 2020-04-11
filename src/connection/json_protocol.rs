use serde::ser::{SerializeStruct, Serializer};
use std::error::Error;

use super::{ObjectId, Protocol, Value};

pub struct JsonProtocol {}

impl JsonProtocol {
    pub fn new() -> Self {
        Self {}
    }
}

impl Protocol for JsonProtocol {
    fn serialize_property_update(
        &self,
        object: ObjectId,
        property: &str,
        value: &Value,
    ) -> Result<Vec<u8>, Box<Error>> {
        let buffer = Vec::with_capacity(128);
        let mut serializer = serde_json::Serializer::new(buffer);
        let mut message = serializer.serialize_struct("Message", 4)?;
        message.serialize_field("mtype", "update")?;
        message.serialize_field("object", &object)?;
        message.serialize_field("property", property)?;
        message.serialize_field("value", value)?;
        message.end()?;
        Ok(serializer.into_inner())
    }
}
