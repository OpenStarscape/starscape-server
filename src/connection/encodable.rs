//! All things relating to encoding (serializing) data that is to be sent to the client

use super::*;

/// A value that can be encoded (aka serialized) and sent to a client. Note that it requires an
/// EncodeCtx in order to be encoded. See bind().
#[derive(Debug, PartialEq, Clone)]
pub enum Encodable {
    Vector(Vector3<f64>),
    Scalar(f64),
    Integer(i64),
    Text(String),
    Entity(EntityKey),
    Array(Vec<Encodable>),
    Null,
    // TODO: add boolean
    // TODO: add map
    // (for each JSON encoding, JSON decoding and Decoded getting needs to be tested)
}

impl From<String> for Encodable {
    fn from(text: String) -> Self {
        Encodable::Text(text)
    }
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
        Encodable::Scalar(value)
    }
}

impl From<f32> for Encodable {
    fn from(value: f32) -> Self {
        Encodable::Scalar(f64::from(value))
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
        Encodable::Array(vec.into_iter().map(From::from).collect())
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
