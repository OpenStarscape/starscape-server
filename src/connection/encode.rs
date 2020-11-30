//! All things relating to encoding (serializing) data that is to be sent to the client

use super::*;

/// A value that can be encoded (aka serialized) and sent to a client. Note that it requires an
/// EncodeCtx in order to be encoded. See bind().
#[derive(Debug, PartialEq, Clone)]
pub enum Encodable {
    Vector(Vector3<f64>),
    Scaler(f64),
    Integer(i64),
    Entity(EntityKey),
    List(Vec<Encodable>),
    Null,
}

impl Encodable {
    /// Allows binding a context to an encodable, which makes it serializable with serde
    pub fn bind<'a>(&'a self, ctx: &'a dyn EncodeCtx) -> ContextualizedEncodable<'a> {
        ContextualizedEncodable {
            encodable: self,
            ctx,
        }
    }
}

/// The required context for encoding. The normal implementation is ObjectMapImpl.
pub trait EncodeCtx {
    /// Returns the object ID for the given entity, creating a new one if needed
    fn object_for(&self, entity: EntityKey) -> ObjectId;
}

/// An encodable attached to a context. This may be serialized with serde.
pub struct ContextualizedEncodable<'a> {
    encodable: &'a Encodable,
    ctx: &'a dyn EncodeCtx,
}

impl From<Point3<f64>> for Encodable {
    fn from(point: Point3<f64>) -> Self {
        Encodable::Vector(point.to_vec())
    }
}

impl From<Vector3<f64>> for Encodable {
    fn from(vector: Vector3<f64>) -> Self {
        Encodable::Vector(vector)
    }
}

impl From<f64> for Encodable {
    fn from(value: f64) -> Self {
        Encodable::Scaler(value)
    }
}

impl From<f32> for Encodable {
    fn from(value: f32) -> Self {
        Encodable::Scaler(f64::from(value))
    }
}

impl From<i64> for Encodable {
    fn from(value: i64) -> Self {
        Encodable::Integer(value)
    }
}

impl From<i32> for Encodable {
    fn from(value: i32) -> Self {
        Encodable::Integer(i64::from(value))
    }
}

impl From<u64> for Encodable {
    fn from(value: u64) -> Self {
        Encodable::Integer(value as i64)
    }
}

impl From<u32> for Encodable {
    fn from(value: u32) -> Self {
        Encodable::Integer(i64::from(value))
    }
}

impl From<EntityKey> for Encodable {
    fn from(entity: EntityKey) -> Self {
        Encodable::Entity(entity)
    }
}

impl<T> From<Vec<T>> for Encodable
where
    Encodable: From<T>,
{
    fn from(vec: Vec<T>) -> Self {
        Encodable::List(vec.into_iter().map(From::from).collect())
    }
}

impl From<()> for Encodable {
    fn from(_: ()) -> Self {
        Encodable::Null
    }
}

impl<T: Into<Encodable>> From<Option<T>> for Encodable {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(value) => value.into(),
            None => Encodable::Null,
        }
    }
}

impl<'a> Serialize for ContextualizedEncodable<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.encodable {
            Encodable::Vector(vector) => {
                use serde::ser::SerializeTuple;
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element(&vector.x)?;
                tuple.serialize_element(&vector.y)?;
                tuple.serialize_element(&vector.z)?;
                tuple.end()
            }
            Encodable::Scaler(value) => serializer.serialize_f64(*value),
            Encodable::Integer(value) => serializer.serialize_i64(*value),
            Encodable::Entity(entity) => serializer.serialize_u64(self.ctx.object_for(*entity)),
            Encodable::List(list) => {
                use serde::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(list.len()))?;
                for elem in list {
                    seq.serialize_element(&elem.bind(self.ctx))?
                }
                seq.end()
            }
            Encodable::Null => serializer.serialize_none(),
        }
    }
}

/// Encodes a specific data format (ex JSON)
/// Any encoder should be compatible with any session (JSON should work with TCP, websockets, etc)
pub trait Encoder {
    fn encode_property_update(
        &self,
        object: ObjectId,
        property: &str,
        ctx: &dyn EncodeCtx,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
}

#[cfg(test)]
mod json_tests {
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
            &serde_json::to_string(&value.bind(&MockEncodeCtx)).expect("failed to serialize"),
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
    fn list_of_ints() {
        assert_json_eq(vec![1, 2, 3, 69, 42].into(), "[1, 2, 3, 69, 42]");
    }

    #[test]
    fn null() {
        assert_json_eq(().into(), "null");
    }

    #[test]
    fn entity() {
        let e: Vec<EntityKey> = mock_keys(1);
        assert_json_eq(e[0].into(), "42"); // MOCK_OBJ_ID
    }

    #[test]
    fn list_of_entities() {
        let e: Vec<EntityKey> = mock_keys(3);
        // the mock context returns MOCK_OBJ_ID no matter what
        assert_json_eq(e.into(), "[42, 42, 42]");
    }
}
