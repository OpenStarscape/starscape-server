use super::*;
use serde::ser::{Serialize, Serializer};

#[derive(Debug, PartialEq, Clone)]
pub enum Encodable {
    Vector(Vector3<f64>),
    Scaler(f64),
    Integer(i64),
    /// Entity needs to be transformed into an object ID before serialization
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

impl Serialize for Encodable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
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
            Encodable::Entity(entity) => {
                panic!(
                    "Can not serialize {:?}; entity should have been replaced by object ID",
                    entity
                );
            }
            Encodable::List(list) => {
                use serde::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(list.len()))?;
                for elem in list {
                    seq.serialize_element(elem)?
                }
                seq.end()
            }
            Encodable::Null => serializer.serialize_none(),
        }
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;

    fn assert_json_eq(value: Encodable, json: &str) {
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
    fn list_of_ints() {
        assert_json_eq(vec![1, 2, 3, 69, 42].into(), "[1, 2, 3, 69, 42]");
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

    #[test]
    #[should_panic]
    fn list_of_entities() {
        let e: Vec<EntityKey> = mock_keys(3);
        assert_json_eq(e.into(), "should panic");
    }
}
