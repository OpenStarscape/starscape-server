use cgmath::*;
use serde::ser::{Serialize, Serializer};
use std::error::Error;
use std::io::Write;

use crate::connection::Connection;
use crate::state::EntityKey;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Vector(Vector3<f64>),
    Scaler(f64),
    Integer(i64),
    /// Entity needs to be transformed into an object ID before serialization
    Entity(EntityKey),
    Null,
}

impl From<Point3<f64>> for Value {
    fn from(point: Point3<f64>) -> Self {
        Value::Vector(point.to_vec())
    }
}

impl From<Vector3<f64>> for Value {
    fn from(vector: Vector3<f64>) -> Self {
        Value::Vector(vector)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Scaler(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Scaler(value as f64)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Integer(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Integer(value as i64)
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value::Integer(value as i64)
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::Integer(value as i64)
    }
}

impl From<EntityKey> for Value {
    fn from(entity: EntityKey) -> Self {
        Value::Entity(entity)
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(value) => value.into(),
            None => Value::Null,
        }
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Vector(vector) => {
                use serde::ser::SerializeTuple;
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element(&vector.x)?;
                tuple.serialize_element(&vector.y)?;
                tuple.serialize_element(&vector.z)?;
                tuple.end()
            }
            Value::Scaler(value) => serializer.serialize_f64(*value),
            Value::Integer(value) => serializer.serialize_i64(*value),
            Value::Entity(entity) => {
                panic!(
                    "Can not serialize {:?}; entity should have been replaced by object ID",
                    entity
                );
            }
            Value::Null => serializer.serialize_none(),
        }
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;
    use crate::state::mock_keys;

    fn assert_json_eq(value: Value, json: &str) {
        let expected: serde_json::Value =
            serde_json::from_str(json).expect("failed to parse test JSON");
        let actual: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&value).expect("Failed to serialize"))
                .expect("Failed to parse the JSON we just generated");
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
    fn null() {
        assert_json_eq(().into(), "null");
    }

    #[test]
    #[should_panic]
    fn entity() {
        let e: Vec<EntityKey> = mock_keys(1);
        assert_json_eq(e[0].into(), "should panic");
    }
}
