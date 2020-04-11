use cgmath::*;
use serde::ser::{Serialize, SerializeTuple, Serializer};
use std::error::Error;
use std::io::Write;

use crate::connection::Connection;
use crate::state::EntityKey;

pub enum Value {
    Vector(Vector3<f64>),
    Scaler(f64),
    Integer(i64),
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
