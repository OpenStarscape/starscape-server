//! All things relating to encoding (serializing) data that is to be sent to the client

use super::*;

/// A protocol value that can be serialized by an Encoder and deserialized by a Decoder.
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Vector(Vector3<f64>),
    Scalar(f64),
    Integer(i64),
    Text(String),
    Entity(EntityKey),
    Array(Vec<Value>),
    Null,
    // TODO: add boolean
    // TODO: add map
    // (for each JSON encoding, JSON decoding and Value getting needs to be tested)
}

impl AssertIsSync for Value {}

pub type DecodeResult<T> = RequestResult<T>;

impl From<String> for Value {
    fn from(text: String) -> Self {
        Value::Text(text)
    }
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
        Value::Scalar(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Scalar(f64::from(value))
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Integer(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Integer(i64::from(value))
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value::Integer(value as i64)
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::Integer(i64::from(value))
    }
}

impl From<EntityKey> for Value {
    fn from(entity: EntityKey) -> Self {
        if entity.is_null() {
            Value::Null
        } else {
            Value::Entity(entity)
        }
    }
}

impl From<ColorRGB> for Value {
    fn from(color: ColorRGB) -> Self {
        Value::Text(format!("0x{:02X}{:02X}{:02X}", color.r, color.g, color.b))
    }
}

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    fn from(vec: Vec<T>) -> Self {
        Value::Array(vec.into_iter().map(Into::into).collect())
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

impl<T> From<Option<T>> for Value
where
    T: Into<Value>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(value) => value.into(),
            None => Value::Null,
        }
    }
}

// TODO: implement tuples with a macro

impl<A> From<(A,)> for Value
where
    A: Into<Value>,
{
    fn from(tuple: (A,)) -> Self {
        Value::Array(vec![tuple.0.into()])
    }
}

impl<A, B> From<(A, B)> for Value
where
    A: Into<Value>,
    B: Into<Value>,
{
    fn from(tuple: (A, B)) -> Self {
        Value::Array(vec![tuple.0.into(), tuple.1.into()])
    }
}

impl<A, B, C> From<(A, B, C)> for Value
where
    A: Into<Value>,
    B: Into<Value>,
    C: Into<Value>,
{
    fn from(tuple: (A, B, C)) -> Self {
        Value::Array(vec![tuple.0.into(), tuple.1.into(), tuple.2.into()])
    }
}

impl<A, B, C, D> From<(A, B, C, D)> for Value
where
    A: Into<Value>,
    B: Into<Value>,
    C: Into<Value>,
    D: Into<Value>,
{
    fn from(tuple: (A, B, C, D)) -> Self {
        Value::Array(vec![
            tuple.0.into(),
            tuple.1.into(),
            tuple.2.into(),
            tuple.3.into(),
        ])
    }
}

impl<A, B, C, D, E> From<(A, B, C, D, E)> for Value
where
    A: Into<Value>,
    B: Into<Value>,
    C: Into<Value>,
    D: Into<Value>,
    E: Into<Value>,
{
    fn from(tuple: (A, B, C, D, E)) -> Self {
        Value::Array(vec![
            tuple.0.into(),
            tuple.1.into(),
            tuple.2.into(),
            tuple.3.into(),
            tuple.4.into(),
        ])
    }
}

impl<A, B, C, D, E, F> From<(A, B, C, D, E, F)> for Value
where
    A: Into<Value>,
    B: Into<Value>,
    C: Into<Value>,
    D: Into<Value>,
    E: Into<Value>,
    F: Into<Value>,
{
    fn from(tuple: (A, B, C, D, E, F)) -> Self {
        Value::Array(vec![
            tuple.0.into(),
            tuple.1.into(),
            tuple.2.into(),
            tuple.3.into(),
            tuple.4.into(),
            tuple.5.into(),
        ])
    }
}

impl<A, B, C, D, E, F, G> From<(A, B, C, D, E, F, G)> for Value
where
    A: Into<Value>,
    B: Into<Value>,
    C: Into<Value>,
    D: Into<Value>,
    E: Into<Value>,
    F: Into<Value>,
    G: Into<Value>,
{
    fn from(tuple: (A, B, C, D, E, F, G)) -> Self {
        Value::Array(vec![
            tuple.0.into(),
            tuple.1.into(),
            tuple.2.into(),
            tuple.3.into(),
            tuple.4.into(),
            tuple.5.into(),
            tuple.6.into(),
        ])
    }
}

impl<A, B, C, D, E, F, G, H> From<(A, B, C, D, E, F, G, H)> for Value
where
    A: Into<Value>,
    B: Into<Value>,
    C: Into<Value>,
    D: Into<Value>,
    E: Into<Value>,
    F: Into<Value>,
    G: Into<Value>,
    H: Into<Value>,
{
    fn from(tuple: (A, B, C, D, E, F, G, H)) -> Self {
        Value::Array(vec![
            tuple.0.into(),
            tuple.1.into(),
            tuple.2.into(),
            tuple.3.into(),
            tuple.4.into(),
            tuple.5.into(),
            tuple.6.into(),
            tuple.7.into(),
        ])
    }
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

impl From<Value> for DecodeResult<Value> {
    fn from(value: Value) -> Self {
        Ok(value)
    }
}

impl From<Value> for DecodeResult<Vector3<f64>> {
    fn from(value: Value) -> Self {
        match value {
            Value::Vector(v) => Ok(v),
            _ => Err(BadRequest(format!("{:?} is not a 3D vector", value))),
        }
    }
}

impl From<Value> for DecodeResult<Point3<f64>> {
    fn from(value: Value) -> Self {
        DecodeResult::<Vector3<f64>>::from(value).map(Point3::from_vec)
    }
}

impl From<Value> for DecodeResult<f64> {
    fn from(value: Value) -> Self {
        match value {
            Value::Scalar(value) => Ok(value),
            Value::Integer(value) => Ok(value as f64),
            _ => Err(BadRequest(format!("{:?} is not a number", value))),
        }
    }
}

impl From<Value> for DecodeResult<i64> {
    fn from(value: Value) -> Self {
        match value {
            Value::Integer(value) => Ok(value),
            Value::Scalar(value) => {
                Err(BadRequest(format!("{} is a scalar, not an integer", value)))
            }
            _ => Err(BadRequest(format!("{:?} is not a number", value))),
        }
    }
}

impl From<Value> for DecodeResult<u64> {
    fn from(value: Value) -> Self {
        use std::convert::TryInto;
        match DecodeResult::<i64>::from(value)?.try_into() {
            Ok(i) => Ok(i),
            Err(e) => Err(BadRequest(format!("{}", e))),
        }
    }
}

impl From<Value> for DecodeResult<String> {
    fn from(value: Value) -> Self {
        match value {
            Value::Text(value) => Ok(value),
            _ => Err(BadRequest(format!("{:?} is not a string", value))),
        }
    }
}

impl From<Value> for DecodeResult<EntityKey> {
    fn from(value: Value) -> Self {
        match value {
            Value::Entity(value) => Ok(value),
            Value::Null => Ok(EntityKey::null()),
            _ => Err(BadRequest(format!("{:?} is not an object", value))),
        }
    }
}

impl From<Value> for DecodeResult<ColorRGB> {
    fn from(value: Value) -> Self {
        let s: String = Into::<DecodeResult<String>>::into(value)?;
        if !s.starts_with("0x") {
            return Err(BadRequest("color does not start with 0x".to_string()));
        }
        let u = u32::from_str_radix(&s[2..], 16)
            .map_err(|e| BadRequest(format!("could not parse color: {}", e)))?;
        if u >> 24 != 0 {
            return Err(BadRequest("color has too many digits".to_string()));
        }
        Ok(ColorRGB::from_u32(u))
    }
}

impl<T> From<Value> for DecodeResult<Vec<T>>
where
    Value: Into<DecodeResult<T>>,
{
    fn from(value: Value) -> Self {
        match value {
            Value::Array(vec) => vec.into_iter().map(Into::into).collect(),
            _ => Err(BadRequest(format!("{:?} is not an array", value))),
        }
    }
}

impl From<Value> for DecodeResult<()> {
    fn from(value: Value) -> Self {
        if value.is_null() {
            Ok(())
        } else {
            Err(BadRequest(format!("{:?} is not null", value)))
        }
    }
}

impl<T> From<Value> for DecodeResult<Option<T>>
where
    Value: Into<DecodeResult<T>>,
{
    fn from(value: Value) -> Self {
        if value.is_null() {
            Ok(None)
        } else {
            Ok(Some(value.into()?))
        }
    }
}

/// TODO: implement all the tuples with a macro

impl<A> From<Value> for DecodeResult<(A,)>
where
    Value: Into<DecodeResult<A>>,
{
    fn from(value: Value) -> Self {
        const LEN: usize = 1;
        let vec = match value {
            Value::Array(vec) => vec,
            _ => {
                return Err(BadRequest(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                )))
            }
        };
        if vec.len() != LEN {
            return Err(BadRequest(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            )));
        }
        let mut iter = vec.into_iter();
        Ok((Into::<DecodeResult<A>>::into(iter.next().unwrap())?,))
    }
}

impl<A, B> From<Value> for DecodeResult<(A, B)>
where
    Value: Into<DecodeResult<A>>,
    Value: Into<DecodeResult<B>>,
{
    fn from(value: Value) -> Self {
        const LEN: usize = 2;
        let vec = match value {
            Value::Array(vec) => vec,
            _ => {
                return Err(BadRequest(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                )))
            }
        };
        if vec.len() != LEN {
            return Err(BadRequest(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            )));
        }
        let mut iter = vec.into_iter();
        Ok((
            Into::<DecodeResult<A>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<B>>::into(iter.next().unwrap())?,
        ))
    }
}

impl<A, B, C> From<Value> for DecodeResult<(A, B, C)>
where
    Value: Into<DecodeResult<A>>,
    Value: Into<DecodeResult<B>>,
    Value: Into<DecodeResult<C>>,
{
    fn from(value: Value) -> Self {
        const LEN: usize = 3;
        let vec = match value {
            Value::Array(vec) => vec,
            _ => {
                return Err(BadRequest(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                )))
            }
        };
        if vec.len() != LEN {
            return Err(BadRequest(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            )));
        }
        let mut iter = vec.into_iter();
        Ok((
            Into::<DecodeResult<A>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<B>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<C>>::into(iter.next().unwrap())?,
        ))
    }
}

impl<A, B, C, D> From<Value> for DecodeResult<(A, B, C, D)>
where
    Value: Into<DecodeResult<A>>,
    Value: Into<DecodeResult<B>>,
    Value: Into<DecodeResult<C>>,
    Value: Into<DecodeResult<D>>,
{
    fn from(value: Value) -> Self {
        const LEN: usize = 4;
        let vec = match value {
            Value::Array(vec) => vec,
            _ => {
                return Err(BadRequest(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                )))
            }
        };
        if vec.len() != LEN {
            return Err(BadRequest(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            )));
        }
        let mut iter = vec.into_iter();
        Ok((
            Into::<DecodeResult<A>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<B>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<C>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<D>>::into(iter.next().unwrap())?,
        ))
    }
}

impl<A, B, C, D, E> From<Value> for DecodeResult<(A, B, C, D, E)>
where
    Value: Into<DecodeResult<A>>,
    Value: Into<DecodeResult<B>>,
    Value: Into<DecodeResult<C>>,
    Value: Into<DecodeResult<D>>,
    Value: Into<DecodeResult<E>>,
{
    fn from(value: Value) -> Self {
        const LEN: usize = 5;
        let vec = match value {
            Value::Array(vec) => vec,
            _ => {
                return Err(BadRequest(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                )))
            }
        };
        if vec.len() != LEN {
            return Err(BadRequest(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            )));
        }
        let mut iter = vec.into_iter();
        Ok((
            Into::<DecodeResult<A>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<B>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<C>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<D>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<E>>::into(iter.next().unwrap())?,
        ))
    }
}

impl<A, B, C, D, E, F> From<Value> for DecodeResult<(A, B, C, D, E, F)>
where
    Value: Into<DecodeResult<A>>,
    Value: Into<DecodeResult<B>>,
    Value: Into<DecodeResult<C>>,
    Value: Into<DecodeResult<D>>,
    Value: Into<DecodeResult<E>>,
    Value: Into<DecodeResult<F>>,
{
    fn from(value: Value) -> Self {
        const LEN: usize = 6;
        let vec = match value {
            Value::Array(vec) => vec,
            _ => {
                return Err(BadRequest(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                )))
            }
        };
        if vec.len() != LEN {
            return Err(BadRequest(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            )));
        }
        let mut iter = vec.into_iter();
        Ok((
            Into::<DecodeResult<A>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<B>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<C>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<D>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<E>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<F>>::into(iter.next().unwrap())?,
        ))
    }
}

impl<A, B, C, D, E, F, G> From<Value> for DecodeResult<(A, B, C, D, E, F, G)>
where
    Value: Into<DecodeResult<A>>,
    Value: Into<DecodeResult<B>>,
    Value: Into<DecodeResult<C>>,
    Value: Into<DecodeResult<D>>,
    Value: Into<DecodeResult<E>>,
    Value: Into<DecodeResult<F>>,
    Value: Into<DecodeResult<G>>,
{
    fn from(value: Value) -> Self {
        const LEN: usize = 7;
        let vec = match value {
            Value::Array(vec) => vec,
            _ => {
                return Err(BadRequest(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                )))
            }
        };
        if vec.len() != LEN {
            return Err(BadRequest(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            )));
        }
        let mut iter = vec.into_iter();
        Ok((
            Into::<DecodeResult<A>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<B>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<C>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<D>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<E>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<F>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<G>>::into(iter.next().unwrap())?,
        ))
    }
}

impl<A, B, C, D, E, F, G, H> From<Value> for DecodeResult<(A, B, C, D, E, F, G, H)>
where
    Value: Into<DecodeResult<A>>,
    Value: Into<DecodeResult<B>>,
    Value: Into<DecodeResult<C>>,
    Value: Into<DecodeResult<D>>,
    Value: Into<DecodeResult<E>>,
    Value: Into<DecodeResult<F>>,
    Value: Into<DecodeResult<G>>,
    Value: Into<DecodeResult<H>>,
{
    fn from(value: Value) -> Self {
        const LEN: usize = 8;
        let vec = match value {
            Value::Array(vec) => vec,
            _ => {
                return Err(BadRequest(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                )))
            }
        };
        if vec.len() != LEN {
            return Err(BadRequest(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            )));
        }
        let mut iter = vec.into_iter();
        Ok((
            Into::<DecodeResult<A>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<B>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<C>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<D>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<E>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<F>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<G>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<H>>::into(iter.next().unwrap())?,
        ))
    }
}

#[cfg(test)]
mod encode_tests {
    use super::*;
    use Value::*;

    #[test]
    fn encodes_color_correctly() {
        let enc: Value = ColorRGB::from_u32(0x0F0080).into();
        assert_eq!(enc, Text("0x0F0080".to_string()));
    }

    #[test]
    fn encodes_null_entity_as_null() {
        use slotmap::Key;
        let enc: Value = EntityKey::null().into();
        assert_eq!(enc, Null);
    }
}

#[cfg(test)]
mod decode_tests {
    use super::*;
    use Value::*;

    fn assert_decodes_to<T>(decodable: Value, expected: T)
    where
        T: PartialEq + Debug,
        DecodeResult<T>: From<Value>,
    {
        let actual: T = DecodeResult::<T>::from(decodable).expect("failed to decode");
        assert_eq!(actual, expected);
    }

    fn assert_doesnt_decode_to<T>(decodable: Value)
    where
        T: PartialEq + Debug,
        DecodeResult<T>: From<Value>,
    {
        assert!(matches!(DecodeResult::<T>::from(decodable), Err(_)));
    }

    #[test]
    fn can_get_decoded() {
        assert_decodes_to::<Value>(Integer(7), Integer(7));
        assert_decodes_to::<Value>(Text("foo".into()), Text("foo".into()));
        assert_decodes_to::<Value>(Array(vec![]), Array(vec![]));
    }

    #[test]
    fn can_get_integer() {
        let i = -5;
        assert_decodes_to::<i64>(Integer(i), i);
    }

    #[test]
    fn can_get_unsigned_from_int() {
        let u = 7;
        assert_decodes_to::<u64>(Integer(7), u);
    }

    #[test]
    fn can_get_float_from_int() {
        let f = 7.0;
        assert_decodes_to::<f64>(Integer(7), f);
    }

    #[test]
    fn can_get_scalar() {
        let f = 7.0;
        assert_decodes_to::<f64>(Scalar(f), f);
    }

    #[test]
    fn can_not_get_int_from_scalar() {
        assert_doesnt_decode_to::<i64>(Scalar(7.0));
    }

    #[test]
    fn can_not_get_unsigned_from_negative_int() {
        assert_doesnt_decode_to::<u64>(Integer(-5))
    }

    #[test]
    fn can_get_vector() {
        let vector = Vector3::new(1.0, 2.5, -3.0);
        let decodable = Vector(vector);
        assert_decodes_to::<Vector3<f64>>(decodable, vector);
    }

    #[test]
    fn can_get_point() {
        let point = Point3::new(1.0, 2.5, -3.0);
        let vector = point.to_vec();
        let decodable = Vector(vector);
        assert_decodes_to::<Point3<f64>>(decodable, point);
    }

    #[test]
    fn can_get_text() {
        assert_decodes_to::<String>(Text("hello".to_string()), "hello".to_string());
    }

    #[test]
    fn can_get_null() {
        assert_decodes_to::<()>(Null, ());
    }

    #[test]
    fn zero_is_not_null() {
        assert_doesnt_decode_to::<()>(Integer(0));
    }

    #[test]
    fn can_get_some_option() {
        let i = 7;
        assert_decodes_to::<Option<i64>>(Integer(7), Some(i));
    }

    #[test]
    fn can_get_none_option() {
        let option = None;
        assert_decodes_to::<Option<i64>>(Null, option);
    }

    #[test]
    fn can_get_entity() {
        let e: Vec<EntityKey> = mock_keys(1);
        assert_decodes_to::<EntityKey>(Entity(e[0]), e[0]);
    }

    #[test]
    fn can_get_null_entity_from_null() {
        use slotmap::Key;
        assert_decodes_to::<EntityKey>(Null, EntityKey::null());
    }

    #[test]
    fn can_get_none_option_entity() {
        assert_decodes_to::<Option<EntityKey>>(Null, None);
    }

    #[test]
    fn can_get_color() {
        let color = ColorRGB::from_u32(0xF801A2);
        assert_decodes_to::<ColorRGB>(Text("0xF801a2".to_string()), color);
    }

    #[test]
    fn can_get_array_of_ints() {
        let values = vec![7, 8, 9];
        assert_decodes_to::<Vec<i64>>(Array(vec![Integer(7), Integer(8), Integer(9)]), values);
    }

    #[test]
    fn can_get_array_of_options() {
        let values = vec![Some(7), Some(8), None, Some(10)];
        assert_decodes_to::<Vec<Option<i64>>>(
            Array(vec![Integer(7), Integer(8), Null, Integer(10)]),
            values,
        );
    }

    #[test]
    fn empty_array_is_not_null() {
        assert_doesnt_decode_to::<()>(Array(vec![]));
    }

    #[test]
    fn single_wrong_type_stops_array_from_decoding() {
        assert_doesnt_decode_to::<Vec<i64>>(Array(vec![Integer(7), Integer(12), Null, Integer(3)]));
    }

    #[test]
    fn can_get_one_tuple() {
        let value = (7,);
        assert_decodes_to::<(i64,)>(Array(vec![Integer(7)]), value);
    }

    #[test]
    fn one_long_array_is_not_inner_value() {
        assert_doesnt_decode_to::<i64>(Array(vec![Integer(7)]));
    }

    #[test]
    fn value_is_not_one_tuple() {
        assert_doesnt_decode_to::<(i64,)>(Integer(7));
    }

    #[test]
    fn can_get_two_tuple() {
        let value = (7, "hello".into());
        assert_decodes_to::<(i64, String)>(Array(vec![Integer(7), Text("hello".into())]), value);
    }

    #[test]
    fn can_get_three_tuple() {
        let value = (7, "hello".into(), None);
        assert_decodes_to::<(i64, String, Option<f64>)>(
            Array(vec![Integer(7), Text("hello".into()), Null]),
            value,
        );
    }

    #[test]
    fn can_get_four_tuple() {
        let value = (7, "hello".into(), None, Point3::new(1.0, 2.0, 3.0));
        assert_decodes_to::<(i64, String, Option<f64>, Point3<f64>)>(
            Array(vec![
                Integer(7),
                Text("hello".into()),
                Null,
                Vector(Vector3::new(1.0, 2.0, 3.0)),
            ]),
            value,
        );
    }

    #[test]
    fn can_get_five_tuple() {
        let value = (7, "hello".into(), None, Point3::new(1.0, 2.0, 3.0), 3.5);
        assert_decodes_to::<(i64, String, Option<f64>, Point3<f64>, f64)>(
            Array(vec![
                Integer(7),
                Text("hello".into()),
                Null,
                Vector(Vector3::new(1.0, 2.0, 3.0)),
                Scalar(3.5),
            ]),
            value,
        );
    }

    #[test]
    fn can_get_six_tuple() {
        let value = (
            7,
            "hello".into(),
            None,
            Point3::new(1.0, 2.0, 3.0),
            3.5,
            Vector3::new(0.0, 0.0, 0.0),
        );
        assert_decodes_to::<(i64, String, Option<f64>, Point3<f64>, f64, Vector3<f64>)>(
            Array(vec![
                Integer(7),
                Text("hello".into()),
                Null,
                Vector(Vector3::new(1.0, 2.0, 3.0)),
                Scalar(3.5),
                Vector(Vector3::new(0.0, 0.0, 0.0)),
            ]),
            value,
        );
    }

    #[test]
    fn can_get_seven_tuple() {
        let value = (
            7,
            "hello".into(),
            None,
            Point3::new(1.0, 2.0, 3.0),
            3.5,
            Vector3::new(0.0, 0.0, 0.0),
            EntityKey::null(),
        );
        assert_decodes_to::<(
            i64,
            String,
            Option<f64>,
            Point3<f64>,
            f64,
            Vector3<f64>,
            EntityKey,
        )>(
            Array(vec![
                Integer(7),
                Text("hello".into()),
                Null,
                Vector(Vector3::new(1.0, 2.0, 3.0)),
                Scalar(3.5),
                Vector(Vector3::new(0.0, 0.0, 0.0)),
                Entity(EntityKey::null()),
            ]),
            value,
        );
    }

    #[test]
    fn can_get_eight_tuple() {
        let value = (
            7,
            "hello".into(),
            None,
            Point3::new(1.0, 2.0, 3.0),
            3.5,
            Vector3::new(0.0, 0.0, 0.0),
            EntityKey::null(),
            4,
        );
        assert_decodes_to::<(
            i64,
            String,
            Option<f64>,
            Point3<f64>,
            f64,
            Vector3<f64>,
            EntityKey,
            i64,
        )>(
            Array(vec![
                Integer(7),
                Text("hello".into()),
                Null,
                Vector(Vector3::new(1.0, 2.0, 3.0)),
                Scalar(3.5),
                Vector(Vector3::new(0.0, 0.0, 0.0)),
                Entity(EntityKey::null()),
                Integer(4),
            ]),
            value,
        );
    }

    #[test]
    fn can_get_tuple_in_tuple() {
        let value = ((7, 8), vec![]);
        assert_decodes_to::<((i64, i64), Vec<String>)>(
            Array(vec![Array(vec![Integer(7), Integer(8)]), Array(vec![])]),
            value,
        );
    }

    #[test]
    fn can_only_get_one_tuple_if_array_that_size() {
        let decoded = Array(vec![Null]);
        assert_doesnt_decode_to::<(Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value)>(
            decoded.clone(),
        );
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value, Value)>(
            decoded,
        );
    }

    #[test]
    fn can_only_get_two_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null]);
        assert_doesnt_decode_to::<(Value,)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value)>(
            decoded.clone(),
        );
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value, Value)>(
            decoded,
        );
    }

    #[test]
    fn can_only_get_three_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null, Null]);
        assert_doesnt_decode_to::<(Value,)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value)>(
            decoded.clone(),
        );
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value, Value)>(
            decoded,
        );
    }

    #[test]
    fn can_only_get_four_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null, Null, Null]);
        assert_doesnt_decode_to::<(Value,)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value)>(
            decoded.clone(),
        );
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value, Value)>(
            decoded,
        );
    }

    #[test]
    fn can_only_get_five_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null, Null, Null, Null]);
        assert_doesnt_decode_to::<(Value,)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value)>(
            decoded.clone(),
        );
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value, Value)>(
            decoded,
        );
    }

    #[test]
    fn can_only_get_six_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null, Null, Null, Null, Null]);
        assert_doesnt_decode_to::<(Value,)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value)>(
            decoded.clone(),
        );
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value, Value)>(
            decoded,
        );
    }

    #[test]
    fn can_only_get_seven_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null, Null, Null, Null, Null, Null]);
        assert_doesnt_decode_to::<(Value,)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value, Value)>(
            decoded,
        );
    }

    #[test]
    fn can_only_get_eight_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null, Null, Null, Null, Null, Null, Null]);
        assert_doesnt_decode_to::<(Value,)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value)>(decoded.clone());
        assert_doesnt_decode_to::<(Value, Value, Value, Value, Value, Value, Value)>(decoded);
    }
}
