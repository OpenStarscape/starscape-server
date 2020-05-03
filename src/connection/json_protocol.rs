use serde::ser::{SerializeStruct, Serializer};
use std::error::Error;

use super::*;

pub struct JsonProtocol {}

impl JsonProtocol {
    pub fn new() -> Self {
        Self {}
    }
}

impl Encoder for JsonProtocol {
    fn encode_property_update(
        &self,
        object: ObjectId,
        property: &str,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
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

#[cfg(test)]
mod json_tests {
    use super::*;

    fn assert_json_eq(message: &[u8], json: &str) {
        let expected: serde_json::Value =
            serde_json::from_str(json).expect("failed to parse test JSON");
        let actual: serde_json::Value =
            serde_json::from_slice(message).expect("Failed to parse the JSON we generated");
        assert_eq!(actual, expected);
    }

    #[test]
    fn basic_property_update() {
        let p = JsonProtocol::new();
        let obj = 42;
        let prop = "foobar";
        let value = Encodable::Scaler(12.5);
        assert_json_eq(
            &p.encode_property_update(obj, prop, &value)
                .expect("Failed to serialize property update"),
            "{
				\"mtype\": \"update\",
				\"object\": 42,
				\"property\": \"foobar\",
				\"value\": 12.5
			}",
        )
    }
}
