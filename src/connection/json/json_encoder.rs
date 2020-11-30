use super::*;
use serde::ser::{SerializeStruct, Serializer};

pub struct JsonEncoder {}

impl JsonEncoder {
    pub fn new() -> Self {
        Self {}
    }
}

impl Encoder for JsonEncoder {
    fn encode_property_update(
        &self,
        object: ObjectId,
        property: &str,
        ctx: &dyn EncodeCtx,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let buffer = Vec::with_capacity(128);
        let mut serializer = serde_json::Serializer::new(buffer);
        let mut message = serializer.serialize_struct("Message", 4)?;
        message.serialize_field("mtype", "update")?;
        message.serialize_field("object", &object)?;
        message.serialize_field("property", property)?;
        message.serialize_field("value", &value.bind(ctx))?;
        message.end()?;
        Ok(serializer.into_inner())
    }
    fn encode_get_response(
        &self,
        object: ObjectId,
        property: &str,
        ctx: &dyn EncodeCtx,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let buffer = Vec::with_capacity(128);
        let mut serializer = serde_json::Serializer::new(buffer);
        let mut message = serializer.serialize_struct("Message", 4)?;
        message.serialize_field("mtype", "value")?;
        message.serialize_field("object", &object)?;
        message.serialize_field("property", property)?;
        message.serialize_field("value", &value.bind(ctx))?;
        message.end()?;
        Ok(serializer.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    struct MockEncoderCtx;

    impl EncodeCtx for MockEncoderCtx {
        fn object_for(&self, _entity: EntityKey) -> ObjectId {
            panic!("unexpected call")
        }
    }

    fn assert_json_eq(message: &[u8], json: &str) {
        let expected: Value = serde_json::from_str(json).expect("failed to parse test JSON");
        let actual: Value =
            serde_json::from_slice(message).expect("failed to parse the JSON we generated");
        assert_eq!(actual, expected);
    }

    #[test]
    fn basic_property_update() {
        let p = JsonEncoder::new();
        let obj = 42;
        let prop = "foobar";
        let value = Encodable::Scaler(12.5);
        assert_json_eq(
            &p.encode_property_update(obj, prop, &MockEncoderCtx, &value)
                .expect("failed to serialize property update"),
            "{
				\"mtype\": \"update\",
				\"object\": 42,
				\"property\": \"foobar\",
				\"value\": 12.5
			}",
        )
    }

    #[test]
    fn basic_property_value() {
        let p = JsonEncoder::new();
        let obj = 8;
        let prop = "abc";
        let value = Encodable::Integer(19);
        assert_json_eq(
            &p.encode_get_response(obj, prop, &MockEncoderCtx, &value)
                .expect("failed to serialize property update"),
            "{
				\"mtype\": \"value\",
				\"object\": 8,
				\"property\": \"abc\",
				\"value\": 19
			}",
        )
    }
}
