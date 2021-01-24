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

/// A value received from a client. Same type as Encodable, but different name used for clearity.
/// Easiest way to use is to simply call .decode() in a context in which the type is implied.
pub type Decoded = Encodable;

pub type DecodeError = String;
pub type DecodeResult<T> = Result<T, DecodeError>;

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

impl Decoded {
    pub fn is_null(&self) -> bool {
        matches!(self, Decoded::Null)
    }
}

impl From<Decoded> for DecodeResult<Decoded> {
    fn from(value: Decoded) -> Self {
        Ok(value)
    }
}

impl From<Decoded> for DecodeResult<Vector3<f64>> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Vector(v) => Ok(v),
            _ => Err(format!("{:?} is not a 3D vector", value)),
        }
    }
}

impl From<Decoded> for DecodeResult<Point3<f64>> {
    fn from(value: Decoded) -> Self {
        DecodeResult::<Vector3<f64>>::from(value).map(Point3::from_vec)
    }
}

impl From<Decoded> for DecodeResult<f64> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Scalar(value) => Ok(value),
            Decoded::Integer(value) => Ok(value as f64),
            _ => Err(format!("{:?} is not a number", value)),
        }
    }
}

impl From<Decoded> for DecodeResult<i64> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Integer(value) => Ok(value),
            Decoded::Scalar(value) => Err(format!("{} is a scalar, not an integer", value)),
            _ => Err(format!("{:?} is not a number", value)),
        }
    }
}

impl From<Decoded> for DecodeResult<u64> {
    fn from(value: Decoded) -> Self {
        use std::convert::TryInto;
        match DecodeResult::<i64>::from(value)?.try_into() {
            Ok(i) => Ok(i),
            Err(e) => Err(format!("{}", e)),
        }
    }
}

impl From<Decoded> for DecodeResult<String> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Text(value) => Ok(value),
            _ => Err(format!("{:?} is not a string", value)),
        }
    }
}

impl From<Decoded> for DecodeResult<EntityKey> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Entity(value) => Ok(value),
            Decoded::Null => Ok(EntityKey::null()),
            _ => Err(format!("{:?} is not an object", value)),
        }
    }
}

impl From<Decoded> for DecodeResult<ColorRGB> {
    fn from(value: Decoded) -> Self {
        let s: String = Into::<DecodeResult<String>>::into(value)?;
        if !s.starts_with("0x") {
            return Err("color does not start with 0x".to_string());
        }
        let u = u32::from_str_radix(&s[2..], 16).map_err(|e| format!("{}", e))?;
        if u >> 24 != 0 {
            return Err("color has too many digits".to_string());
        }
        Ok(ColorRGB::from_u32(u))
    }
}

impl<T> From<Decoded> for DecodeResult<Vec<T>>
where
    Decoded: Into<DecodeResult<T>>,
{
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Array(vec) => vec.into_iter().map(Into::into).collect(),
            _ => Err(format!("{:?} is not an array", value)),
        }
    }
}

impl From<Decoded> for DecodeResult<()> {
    fn from(value: Decoded) -> Self {
        if value.is_null() {
            Ok(())
        } else {
            Err(format!("{:?} is not null", value))
        }
    }
}

impl<T> From<Decoded> for DecodeResult<Option<T>>
where
    Decoded: Into<DecodeResult<T>>,
{
    fn from(value: Decoded) -> Self {
        if value.is_null() {
            Ok(None)
        } else {
            Ok(Some(value.into()?))
        }
    }
}

/// TODO: implement all the tuples with a macro

impl<A> From<Decoded> for DecodeResult<(A,)>
where
    Decoded: Into<DecodeResult<A>>,
{
    fn from(value: Decoded) -> Self {
        const LEN: usize = 1;
        let vec = match value {
            Decoded::Array(vec) => vec,
            _ => {
                return Err(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                ))
            }
        };
        if vec.len() != LEN {
            return Err(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            ));
        }
        let mut iter = vec.into_iter();
        Ok((Into::<DecodeResult<A>>::into(iter.next().unwrap())?,))
    }
}

impl<A, B> From<Decoded> for DecodeResult<(A, B)>
where
    Decoded: Into<DecodeResult<A>>,
    Decoded: Into<DecodeResult<B>>,
{
    fn from(value: Decoded) -> Self {
        const LEN: usize = 2;
        let vec = match value {
            Decoded::Array(vec) => vec,
            _ => {
                return Err(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                ))
            }
        };
        if vec.len() != LEN {
            return Err(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            ));
        }
        let mut iter = vec.into_iter();
        Ok((
            Into::<DecodeResult<A>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<B>>::into(iter.next().unwrap())?,
        ))
    }
}

impl<A, B, C> From<Decoded> for DecodeResult<(A, B, C)>
where
    Decoded: Into<DecodeResult<A>>,
    Decoded: Into<DecodeResult<B>>,
    Decoded: Into<DecodeResult<C>>,
{
    fn from(value: Decoded) -> Self {
        const LEN: usize = 3;
        let vec = match value {
            Decoded::Array(vec) => vec,
            _ => {
                return Err(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                ))
            }
        };
        if vec.len() != LEN {
            return Err(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            ));
        }
        let mut iter = vec.into_iter();
        Ok((
            Into::<DecodeResult<A>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<B>>::into(iter.next().unwrap())?,
            Into::<DecodeResult<C>>::into(iter.next().unwrap())?,
        ))
    }
}

impl<A, B, C, D> From<Decoded> for DecodeResult<(A, B, C, D)>
where
    Decoded: Into<DecodeResult<A>>,
    Decoded: Into<DecodeResult<B>>,
    Decoded: Into<DecodeResult<C>>,
    Decoded: Into<DecodeResult<D>>,
{
    fn from(value: Decoded) -> Self {
        const LEN: usize = 4;
        let vec = match value {
            Decoded::Array(vec) => vec,
            _ => {
                return Err(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                ))
            }
        };
        if vec.len() != LEN {
            return Err(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            ));
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

impl<A, B, C, D, E> From<Decoded> for DecodeResult<(A, B, C, D, E)>
where
    Decoded: Into<DecodeResult<A>>,
    Decoded: Into<DecodeResult<B>>,
    Decoded: Into<DecodeResult<C>>,
    Decoded: Into<DecodeResult<D>>,
    Decoded: Into<DecodeResult<E>>,
{
    fn from(value: Decoded) -> Self {
        const LEN: usize = 5;
        let vec = match value {
            Decoded::Array(vec) => vec,
            _ => {
                return Err(format!(
                    "can not decode {:?} as a tuple because it is not an array",
                    value
                ))
            }
        };
        if vec.len() != LEN {
            return Err(format!(
                "{:?} has {} elements instead of {}",
                vec,
                vec.len(),
                LEN
            ));
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

#[cfg(test)]
mod encode_tests {
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

#[cfg(test)]
mod decode_tests {
    use super::*;
    use Encodable::*;

    fn assert_decodes_to<T>(decodable: Decoded, expected: T)
    where
        T: PartialEq + Debug,
        DecodeResult<T>: From<Decoded>,
    {
        let actual: T = DecodeResult::<T>::from(decodable).expect("failed to decode");
        assert_eq!(actual, expected);
    }

    fn assert_doesnt_decode_to<T>(decodable: Decoded)
    where
        T: PartialEq + Debug,
        DecodeResult<T>: From<Decoded>,
    {
        assert!(matches!(DecodeResult::<T>::from(decodable), Err(_)));
    }

    #[test]
    fn can_get_decoded() {
        assert_decodes_to::<Decoded>(Integer(7), Integer(7));
        assert_decodes_to::<Decoded>(Text("foo".into()), Text("foo".into()));
        assert_decodes_to::<Decoded>(Array(vec![]), Array(vec![]));
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
        assert_doesnt_decode_to::<(Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded, Decoded, Decoded)>(decoded);
    }

    #[test]
    fn can_only_get_two_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null]);
        assert_doesnt_decode_to::<(Decoded,)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded, Decoded, Decoded)>(decoded);
    }

    #[test]
    fn can_only_get_three_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null, Null]);
        assert_doesnt_decode_to::<(Decoded,)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded, Decoded, Decoded)>(decoded);
    }

    #[test]
    fn can_only_get_four_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null, Null, Null]);
        assert_doesnt_decode_to::<(Decoded,)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded, Decoded, Decoded)>(decoded);
    }

    #[test]
    fn can_only_get_five_tuple_if_array_that_size() {
        let decoded = Array(vec![Null, Null, Null, Null, Null]);
        assert_doesnt_decode_to::<(Decoded,)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded)>(decoded.clone());
        assert_doesnt_decode_to::<(Decoded, Decoded, Decoded, Decoded)>(decoded);
    }
}
