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
        if entity.is_null() {
            Encodable::Null
        } else {
            Encodable::Entity(entity)
        }
    }
}

impl From<ColorRGB> for Encodable {
    fn from(color: ColorRGB) -> Self {
        Encodable::Text(format!("0x{:02X}{:02X}{:02X}", color.r, color.g, color.b))
    }
}

impl<T> From<Vec<T>> for Encodable
where
    T: Into<Encodable>,
{
    fn from(vec: Vec<T>) -> Self {
        Encodable::Array(vec.into_iter().map(Into::into).collect())
    }
}

impl From<()> for Encodable {
    fn from(_: ()) -> Self {
        Encodable::Null
    }
}

impl<T> From<Option<T>> for Encodable
where
    T: Into<Encodable>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(value) => value.into(),
            None => Encodable::Null,
        }
    }
}

// TODO: implement tuples with a macro

impl<A> From<(A,)> for Encodable
where
    A: Into<Encodable>,
{
    fn from(tuple: (A,)) -> Self {
        Encodable::Array(vec![tuple.0.into()])
    }
}

impl<A, B> From<(A, B)> for Encodable
where
    A: Into<Encodable>,
    B: Into<Encodable>,
{
    fn from(tuple: (A, B)) -> Self {
        Encodable::Array(vec![tuple.0.into(), tuple.1.into()])
    }
}

impl<A, B, C> From<(A, B, C)> for Encodable
where
    A: Into<Encodable>,
    B: Into<Encodable>,
    C: Into<Encodable>,
{
    fn from(tuple: (A, B, C)) -> Self {
        Encodable::Array(vec![tuple.0.into(), tuple.1.into(), tuple.2.into()])
    }
}

impl<A, B, C, D> From<(A, B, C, D)> for Encodable
where
    A: Into<Encodable>,
    B: Into<Encodable>,
    C: Into<Encodable>,
    D: Into<Encodable>,
{
    fn from(tuple: (A, B, C, D)) -> Self {
        Encodable::Array(vec![
            tuple.0.into(),
            tuple.1.into(),
            tuple.2.into(),
            tuple.3.into(),
        ])
    }
}

impl<A, B, C, D, E> From<(A, B, C, D, E)> for Encodable
where
    A: Into<Encodable>,
    B: Into<Encodable>,
    C: Into<Encodable>,
    D: Into<Encodable>,
    E: Into<Encodable>,
{
    fn from(tuple: (A, B, C, D, E)) -> Self {
        Encodable::Array(vec![
            tuple.0.into(),
            tuple.1.into(),
            tuple.2.into(),
            tuple.3.into(),
            tuple.4.into(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Encodable::*;

    #[test]
    fn encodes_color_correctly() {
        let enc: Encodable = ColorRGB::from_u32(0x0F0080).into();
        assert_eq!(enc, Text("0x0F0080".to_string()));
    }

    #[test]
    fn encodes_null_entity_as_null() {
        use slotmap::Key;
        let enc: Encodable = EntityKey::null().into();
        assert_eq!(enc, Null);
    }
}
