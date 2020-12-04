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

/// The required context for encoding. The normal implementation is ObjectMapImpl.
pub trait EncodeCtx {
    /// Returns the object ID for the given entity, creating a new one if needed
    fn object_for(&self, entity: EntityKey) -> ObjectId;
}

/// Encodes a specific data format (ex JSON)
/// Any encoder should be compatible with any session (JSON should work with TCP, websockets, etc)
pub trait Encoder {
    /// An update to a subscribed property resulting from a change
    fn encode_property_update(
        &self,
        object: ObjectId,
        property: &str,
        ctx: &dyn EncodeCtx,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
    /// A response to a clients get requst on a property
    fn encode_get_response(
        &self,
        object: ObjectId,
        property: &str,
        ctx: &dyn EncodeCtx,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
}
