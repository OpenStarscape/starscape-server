use cgmath::{Point3, Vector3};
use serde::ser::{Serialize, SerializeTuple, Serializer};

/// Because we can't
pub struct Wrapper<T> {
    inner: T,
}

impl<T> Wrapper<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl Serialize for Wrapper<i32> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

impl<T: Serialize> Serialize for Wrapper<Vector3<T>> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut tuple = serializer.serialize_tuple(3)?;
        tuple.serialize_element(&self.inner.x)?;
        tuple.serialize_element(&self.inner.y)?;
        tuple.serialize_element(&self.inner.z)?;
        tuple.end()
    }
}

impl<T: Serialize> Serialize for Wrapper<Point3<T>> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut tuple = serializer.serialize_tuple(3)?;
        tuple.serialize_element(&self.inner.x)?;
        tuple.serialize_element(&self.inner.y)?;
        tuple.serialize_element(&self.inner.z)?;
        tuple.end()
    }
}

// TODO: make generic so testing is easier
