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

impl<'a> Serialize for Contextualized<'a, Value> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.value {
            Value::Vector(vector) => {
                use serde::ser::SerializeTuple;
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element(&vector.x)?;
                tuple.serialize_element(&vector.y)?;
                tuple.serialize_element(&vector.z)?;
                tuple.end()
            }
            Value::Scalar(value) => serializer.serialize_f64(*value),
            Value::Integer(value) => serializer.serialize_i64(*value),
            Value::Text(value) => serializer.serialize_str(value),
            Value::Entity(entity) => {
                use serde::ser::SerializeTuple;
                let mut outer = serializer.serialize_tuple(1)?;
                outer.serialize_element(
                    &self
                        .ctx
                        .object_for(*entity)
                        .map_err(serde::ser::Error::custom)?,
                )?;
                outer.end()
            }
            Value::Object(_id) => {
                panic!("object serialization not implemented");
            }
            Value::Array(list) => {
                use serde::ser::SerializeTuple;
                let mut outer = serializer.serialize_tuple(1)?;
                outer.serialize_element(&Contextualized::new(list, self.ctx))?;
                outer.end()
            }
            Value::Null => serializer.serialize_none(),
        }
    }
}

/// serde is so annoying
impl<'a> Serialize for Contextualized<'a, Vec<Value>> {
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
    fn encode_event(&self, ctx: &dyn EncodeCtx, event: &Event) -> Result<Vec<u8>, Box<dyn Error>> {
        // TODO: why aren't we reusing buffers?
        let buffer = Vec::with_capacity(128);
        let mut serializer = serde_json::Serializer::new(buffer);
        let mut message = serializer.serialize_map(None)?;
        match event {
            Event::Method(entity, member, method, value) => {
                message.serialize_field(
                    "mtype",
                    match method {
                        EventMethod::Value => "value",
                        EventMethod::Update => "update",
                        EventMethod::Signal => "event",
                    },
                )?;
                message.serialize_field("object", &ctx.object_for(*entity)?)?;
                message.serialize_field("property", member)?;
                message.serialize_field("value", &Contextualized::new(value, ctx))?;
            }
            Event::Destroyed(entity) => {
                message.serialize_field("mtype", "destroyed")?;
                message.serialize_field("object", &ctx.object_for(*entity)?)?;
            }
            Event::FatalError(text) => {
                message.serialize_field("mtype", "error")?;
                message.serialize_field("text", text)?;
            }
        }
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
        fn object_for(&self, _entity: EntityKey) -> RequestResult<ObjectId> {
            Ok(MOCK_OBJ_ID)
        }
    }

    fn assert_json_eq(value: Value, json: &str) {
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
        let i: i64 = -243;
        assert_json_eq(i.into(), "-243");
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

    struct MockEncoderCtx;

    impl EncodeCtx for MockEncoderCtx {
        fn object_for(&self, _entity: EntityKey) -> RequestResult<ObjectId> {
            Ok(42)
        }
    }

    fn assert_json_eq(message: &[u8], json: &str) {
        let expected: serde_json::Value =
            serde_json::from_str(json).expect("failed to parse test JSON");
        let actual: serde_json::Value =
            serde_json::from_slice(message).expect("failed to parse the JSON we generated");
        assert_eq!(actual, expected);
    }

    #[test]
    fn basic_property_update() {
        let p = JsonEncoder::new();
        let e = mock_keys(1);
        let prop = "foobar".to_string();
        let value = Value::Scalar(12.5);
        assert_json_eq(
            &p.encode_event(&MockEncoderCtx, &Event::update(e[0], prop, value))
                .unwrap(),
            "{
                \"mtype\": \"update\",
                \"object\": 42,
                \"property\": \"foobar\",
                \"value\": 12.5
            }",
        )
    }

    #[test]
    fn property_update_with_obj() {
        let p = JsonEncoder::new();
        let e = mock_keys(1);
        let prop = "foobar".to_string();
        let value = Value::Entity(e[0]);
        assert_json_eq(
            &p.encode_event(&MockEncoderCtx, &Event::update(e[0], prop, value))
                .unwrap(),
            "{
                \"mtype\": \"update\",
                \"object\": 42,
                \"property\": \"foobar\",
                \"value\": [42]
            }",
        )
    }

    #[test]
    fn basic_property_value() {
        let p = JsonEncoder::new();
        let e = mock_keys(1);
        let prop = "abc".to_string();
        let value = Value::Integer(19);
        assert_json_eq(
            &p.encode_event(&MockEncoderCtx, &Event::value(e[0], prop, value))
                .unwrap(),
            "{
                \"mtype\": \"value\",
                \"object\": 42,
                \"property\": \"abc\",
                \"value\": 19
            }",
        )
    }

    #[test]
    fn basic_signal() {
        let p = JsonEncoder::new();
        let e = mock_keys(1);
        let prop = "abc".to_string();
        let value = Value::Text("hello".to_string());
        assert_json_eq(
            &p.encode_event(&MockEncoderCtx, &Event::signal(e[0], prop, value))
                .unwrap(),
            "{
                \"mtype\": \"event\",
                \"object\": 42,
                \"property\": \"abc\",
                \"value\": \"hello\"
            }",
        )
    }

    #[test]
    fn entity_destroyed() {
        let p = JsonEncoder::new();
        let e = mock_keys(1);
        assert_json_eq(
            &p.encode_event(&MockEncoderCtx, &Event::Destroyed(e[0]))
                .unwrap(),
            "{
                \"mtype\": \"destroyed\",
                \"object\": 42
            }",
        )
    }

    #[test]
    fn fatal_error() {
        let p = JsonEncoder::new();
        let message = "Error Message".to_string();
        assert_json_eq(
            &p.encode_event(&MockEncoderCtx, &Event::FatalError(message))
                .unwrap(),
            "{
                \"mtype\": \"error\",
                \"text\": \"Error Message\"
            }",
        )
    }
}
