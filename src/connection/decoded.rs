//! All things related to decoding (deserializing) data from the client

use super::*;

/// A value received from a client. Same type as Encodable, but different name used for clearity.
/// Easiest way to use is to simply call .decode() in a context in which the type is implied.
pub type Decoded = Encodable;

pub type DecodedError = String;
pub type DecodedResult<T> = Result<T, DecodedError>;

impl Decoded {
    pub fn is_null(&self) -> bool {
        matches!(self, Decoded::Null)
    }
}

impl From<Decoded> for DecodedResult<Vector3<f64>> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Vector(v) => Ok(v),
            _ => Err(format!("{:?} is not a 3D vector", value)),
        }
    }
}

impl From<Decoded> for DecodedResult<Point3<f64>> {
    fn from(value: Decoded) -> Self {
        DecodedResult::<Vector3<f64>>::from(value).map(Point3::from_vec)
    }
}

impl From<Decoded> for DecodedResult<f64> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Scalar(value) => Ok(value),
            Decoded::Integer(value) => Ok(value as f64),
            _ => Err(format!("{:?} is not a number", value)),
        }
    }
}

impl From<Decoded> for DecodedResult<i64> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Integer(value) => Ok(value),
            Decoded::Scalar(value) => Err(format!("{} is a scalar, not an integer", value)),
            _ => Err(format!("{:?} is not a number", value)),
        }
    }
}

impl From<Decoded> for DecodedResult<u64> {
    fn from(value: Decoded) -> Self {
        use std::convert::TryInto;
        match DecodedResult::<i64>::from(value)?.try_into() {
            Ok(i) => Ok(i),
            Err(e) => Err(format!("{}", e)),
        }
    }
}

impl From<Decoded> for DecodedResult<String> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Text(value) => Ok(value),
            _ => Err(format!("{:?} is not a string", value)),
        }
    }
}

impl From<Decoded> for DecodedResult<EntityKey> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Entity(value) => Ok(value),
            _ => Err(format!("{:?} is not an object", value)),
        }
    }
}

impl From<Decoded> for DecodedResult<Vec<Decoded>> {
    fn from(value: Decoded) -> Self {
        match value {
            Decoded::Array(value) => Ok(value),
            _ => Err(format!("{:?} is not an array", value)),
        }
    }
}

impl From<Decoded> for DecodedResult<()> {
    fn from(value: Decoded) -> Self {
        if value.is_null() {
            Ok(())
        } else {
            Err(format!("{:?} is not null", value))
        }
    }
}

impl From<Decoded> for DecodedResult<Option<Decoded>> {
    fn from(value: Decoded) -> Self {
        if value.is_null() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;
    use Encodable::*;

    fn assert_decodes_to<T>(decodable: Decoded, expected: T)
    where
        T: PartialEq + Debug,
        DecodedResult<T>: From<Decoded>,
    {
        let actual: T = DecodedResult::<T>::from(decodable).expect("failed to decode");
        assert_eq!(actual, expected);
    }

    fn assert_doesnt_decode_to<T>(decodable: Decoded)
    where
        T: PartialEq + Debug,
        DecodedResult<T>: From<Decoded>,
    {
        assert!(matches!(DecodedResult::<T>::from(decodable), Err(_)));
    }

    #[test]
    fn can_get_integer() {
        let i: i64 = -5;
        assert_decodes_to::<i64>(Integer(i), i);
    }

    #[test]
    fn can_get_unsigned_from_int() {
        let u: u64 = 7;
        assert_decodes_to::<u64>(Integer(7), u);
    }

    #[test]
    fn can_get_float_from_int() {
        let f: f64 = 7.0;
        assert_decodes_to::<f64>(Integer(7), f);
    }

    #[test]
    fn can_get_scalar() {
        let f: f64 = 7.0;
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
    fn can_get_some_option_decoded() {
        let i: i64 = 7;
        assert_decodes_to::<Option<Decoded>>(Integer(7), Some(Integer(i)));
    }

    #[test]
    fn can_get_none_option_decoded() {
        let option: Option<Decoded> = None;
        assert_decodes_to::<Option<Decoded>>(Null, option);
    }

    #[test]
    fn can_get_entity() {
        let e: Vec<EntityKey> = mock_keys(1);
        assert_decodes_to::<EntityKey>(Entity(e[0]), e[0]);
    }

    #[test]
    fn can_get_array_of_ints() {
        let values = vec![7, 8, 9];
        let decodable = Array(values.iter().map(|i| Integer(*i)).collect());
        let result: Vec<i64> = DecodedResult::<Vec<Decoded>>::from(decodable)
            .expect("failed to decode as array")
            .into_iter()
            .map(|d| {
                DecodedResult::<i64>::from(d).expect("failed to decode array element as integer")
            })
            .collect();
        assert_eq!(result, values);
    }
}
