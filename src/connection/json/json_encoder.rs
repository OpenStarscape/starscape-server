use super::*;
use serde::ser::{Serialize, SerializeStruct, Serializer};

/// The thing we want to serialize attached to a context. This wrapper is serializable with serde.
struct Contextualized<'a, T> {
    value: &'a T,
    ctx: &'a dyn EncodeCtx,
}

impl<'a, T> Contextualized<'a, T> {
    fn new(value: &'a T, ctx: &'a dyn EncodeCtx) -> Self {
        Self { value, ctx }
    }
}

impl<'a> Serialize for Contextualized<'a, Encodable> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.value {
            Encodable::Vector(vector) => {
                use serde::ser::SerializeTuple;
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element(&vector.x)?;
                tuple.serialize_element(&vector.y)?;
                tuple.serialize_element(&vector.z)?;
                tuple.end()
            }
            Encodable::Scalar(value) => serializer.serialize_f64(*value),
            Encodable::Integer(value) => serializer.serialize_i64(*value),
            Encodable::Text(value) => serializer.serialize_str(value),
            Encodable::Entity(entity) => {
                use serde::ser::SerializeTuple;
                let mut outer = serializer.serialize_tuple(1)?;
                outer.serialize_element(&self.ctx.object_for(*entity))?;
                outer.end()
            }
            Encodable::Array(list) => {
                use serde::ser::SerializeTuple;
                let mut outer = serializer.serialize_tuple(1)?;
                outer.serialize_element(&Contextualized::new(list, self.ctx))?;
                outer.end()
            }
            Encodable::Null => serializer.serialize_none(),
        }
    }
}

/// serde is so annoying
impl<'a> Serialize for Contextualized<'a, Vec<Encodable>> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.value.len()))?;
        for elem in self.value {
            seq.serialize_element(&Contextualized::new(elem, self.ctx))?
        }
        seq.end()
    }
}

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
        // TODO: why aren't we reusing buffers?
        let buffer = Vec::with_capacity(128);
        let mut serializer = serde_json::Serializer::new(buffer);
        let mut message = serializer.serialize_struct("Message", 4)?;
        message.serialize_field("mtype", "update")?;
        message.serialize_field("object", &object)?;
        message.serialize_field("property", property)?;
        message.serialize_field("value", &Contextualized::new(value, ctx))?;
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
        message.serialize_field("value", &Contextualized::new(value, ctx))?;
        message.end()?;
        Ok(serializer.into_inner())
    }

    fn encode_event(
        &self,
        object: ObjectId,
        property: &str,
        ctx: &dyn EncodeCtx,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let buffer = Vec::with_capacity(128);
        let mut serializer = serde_json::Serializer::new(buffer);
        let mut message = serializer.serialize_struct("Message", 4)?;
        message.serialize_field("mtype", "event")?;
        message.serialize_field("object", &object)?;
        message.serialize_field("property", property)?;
        message.serialize_field("value", &Contextualized::new(value, ctx))?;
        message.end()?;
        Ok(serializer.into_inner())
    }

    fn encode_error(&self, text: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let buffer = Vec::with_capacity(text.len() + 32);
        let mut serializer = serde_json::Serializer::new(buffer);
        let mut message = serializer.serialize_struct("Message", 2)?;
        message.serialize_field("mtype", "error")?;
        message.serialize_field("text", text)?;
        message.end()?;
        Ok(serializer.into_inner())
    }
}

#[cfg(test)]
mod encodable_tests {
    use super::*;

    const MOCK_OBJ_ID: ObjectId = 42;

    struct MockEncodeCtx;

    impl EncodeCtx for MockEncodeCtx {
        fn object_for(&self, _entity: EntityKey) -> ObjectId {
            MOCK_OBJ_ID
        }
    }

    fn assert_json_eq(value: Encodable, json: &str) {
        let expected: serde_json::Value =
            serde_json::from_str(json).expect("failed to parse test JSON");
        let actual: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&Contextualized::new(&value, &MockEncodeCtx))
                .expect("failed to serialize"),
        )
        .expect("failed to parse the JSON we just generated");
        assert_eq!(actual, expected);
    }

    #[test]
    fn point() {
        assert_json_eq(Point3::new(1.0, 0.0, -3.0).into(), "[1.0, 0.0, -3.0]")
    }

    #[test]
    fn vector() {
        assert_json_eq(Vector3::new(1.0, 0.0, -3.0).into(), "[1.0, 0.0, -3.0]")
    }

    #[test]
    fn float() {
        assert_json_eq(4.9.into(), "4.9");
    }

    #[test]
    fn int() {
        assert_json_eq((-243 as i64).into(), "-243");
    }

    #[test]
    fn string() {
        assert_json_eq(("hello\n".to_string()).into(), "\"hello\\n\"");
    }

    #[test]
    fn list_of_ints() {
        // Wrapped in an additional list to keep it unambiguous with object IDs and vectors
        assert_json_eq(vec![1, 2, 3, 69, 42].into(), "[[1, 2, 3, 69, 42]]");
    }

    #[test]
    fn null() {
        assert_json_eq(().into(), "null");
    }

    #[test]
    fn entity() {
        let e: Vec<EntityKey> = mock_keys(1);
        assert_json_eq(e[0].into(), "[42]"); // MOCK_OBJ_ID
    }

    #[test]
    fn list_of_entities() {
        let e: Vec<EntityKey> = mock_keys(3);
        // the mock context returns MOCK_OBJ_ID no matter what
        assert_json_eq(e.into(), "[[[42], [42], [42]]]");
    }
}

#[cfg(test)]
mod message_tests {
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
        let value = Encodable::Scalar(12.5);
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
