//! All things related to decoding (deserializing) data from the client

use super::*;

/// A value received from a client. Same type as Encodable, but different name used for clearity.
/// Easiest way to use is to simply call .decode() in a context in which the type is implied.
pub type Decoded = Encodable;

pub type DecodedResult<T> = Result<T, String>;

pub trait GetDecoded<'a, T> {
    fn try_get(&'a self) -> DecodedResult<T>;
}

impl Decoded {
    pub fn vector(&self) -> DecodedResult<Vector3<f64>> {
        match self {
            Decoded::Vector(v) => Ok(*v),
            _ => Err(format!("{:?} is not a 3D vector", self)),
        }
    }

    pub fn scalar(&self) -> DecodedResult<f64> {
        match self {
            Decoded::Scalar(value) => Ok(*value),
            Decoded::Integer(value) => Ok(*value as f64),
            _ => Err(format!("{:?} is not a number", self)),
        }
    }

    pub fn integer(&self) -> DecodedResult<i64> {
        match self {
            Decoded::Integer(value) => Ok(*value),
            Decoded::Scalar(value) => Err(format!("{} is a scalar, not an integer", value)),
            _ => Err(format!("{:?} is not a number", self)),
        }
    }

    pub fn entity(&self) -> DecodedResult<&EntityKey> {
        match self {
            Decoded::Entity(value) => Ok(value),
            _ => Err(format!("{:?} is not an object", self)),
        }
    }

    #[allow(dead_code)]
    pub fn array(&self) -> DecodedResult<&[Decoded]> {
        match self {
            Decoded::Array(list) => Ok(list),
            _ => Err(format!("{:?} is not an array", self)),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Decoded::Null)
    }
}

impl GetDecoded<'_, Vector3<f64>> for Decoded {
    fn try_get(&self) -> DecodedResult<Vector3<f64>> {
        self.vector()
    }
}

impl GetDecoded<'_, Point3<f64>> for Decoded {
    fn try_get(&self) -> DecodedResult<Point3<f64>> {
        self.vector().map(Point3::from_vec)
    }
}

impl GetDecoded<'_, f64> for Decoded {
    fn try_get(&self) -> DecodedResult<f64> {
        self.scalar()
    }
}

impl GetDecoded<'_, i64> for Decoded {
    fn try_get(&self) -> DecodedResult<i64> {
        self.integer()
    }
}

impl GetDecoded<'_, u64> for Decoded {
    fn try_get(&self) -> DecodedResult<u64> {
        match self.integer().map(std::convert::TryFrom::try_from) {
            Ok(Ok(i)) => Ok(i),
            _ => Err(format!("{:?} is not an unsigned integer", self)),
        }
    }
}

impl<'a> GetDecoded<'a, &'a EntityKey> for Decoded {
    fn try_get(&'a self) -> DecodedResult<&'a EntityKey> {
        self.entity()
    }
}

impl<'a> GetDecoded<'a, &'a [Decoded]> for Decoded {
    fn try_get(&'a self) -> DecodedResult<&'a [Decoded]> {
        self.array()
    }
}

impl GetDecoded<'_, ()> for Decoded {
    fn try_get(&self) -> Result<(), String> {
        if self.is_null() {
            Ok(())
        } else {
            Err(format!("{:?} is not null", self))
        }
    }
}

impl<'a, T> GetDecoded<'a, Option<T>> for Decoded
where
    Decoded: GetDecoded<'a, T>,
{
    fn try_get(&'a self) -> DecodedResult<Option<T>> {
        if self.is_null() {
            Ok(None)
        } else {
            Ok(Some(self.try_get()?))
        }
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;
    use Encodable::*;

    fn assert_decodes_to<'a, T>(decodable: &'a Decoded, expected: T)
    where
        T: PartialEq + Debug,
        Decoded: GetDecoded<'a, T>,
    {
        let actual: T = decodable.try_get().expect("failed to decode");
        assert_eq!(actual, expected);
    }

    fn assert_doesnt_decode_to<'a, T>(decodable: &'a Decoded)
    where
        T: PartialEq + Debug,
        Decoded: GetDecoded<'a, T>,
    {
        let actual: DecodedResult<T> = decodable.try_get();
        assert!(matches!(actual, Err(_)));
    }

    #[test]
    fn can_get_integer() {
        let i: i64 = -5;
        assert_decodes_to(&Integer(i), i);
    }

    #[test]
    fn can_get_unsigned_from_int() {
        let u: u64 = 7;
        assert_decodes_to(&Integer(7), u);
    }

    #[test]
    fn can_get_float_from_int() {
        let f: f64 = 7.0;
        assert_decodes_to(&Integer(7), f);
    }

    #[test]
    fn can_get_scalar() {
        let f: f64 = 7.0;
        assert_decodes_to(&Scalar(f), f);
    }

    #[test]
    fn can_not_get_int_from_scalar() {
        assert_doesnt_decode_to::<i64>(&Scalar(7.0));
    }

    #[test]
    fn can_not_get_unsigned_from_negative_int() {
        assert_doesnt_decode_to::<u64>(&Integer(-5))
    }

    #[test]
    fn can_get_vector() {
        let vector = Vector3::new(1.0, 2.5, -3.0);
        let decodable = Vector(vector);
        assert_decodes_to(&decodable, vector);
    }

    #[test]
    fn can_get_point() {
        let point = Point3::new(1.0, 2.5, -3.0);
        let vector = point.to_vec();
        let decodable = Vector(vector);
        assert_decodes_to(&decodable, point);
    }

    #[test]
    fn can_get_null() {
        assert_decodes_to(&Null, ());
    }

    #[test]
    fn zero_is_not_null() {
        assert_doesnt_decode_to::<()>(&Integer(0));
    }

    #[test]
    fn can_get_some_option() {
        let i: i64 = 7;
        assert_decodes_to(&Integer(7), Some(i));
    }

    #[test]
    fn can_get_none_option() {
        let option: Option<i64> = None;
        assert_decodes_to(&Null, option);
    }

    #[test]
    fn can_get_entity() {
        let e: Vec<EntityKey> = mock_keys(1);
        assert_decodes_to(&Entity(e[0]), &e[0]);
    }

    #[test]
    fn can_get_array_of_ints() {
        let values = vec![7, 8, 9];
        let decodable = Array(values.iter().map(|i| Integer(*i)).collect());
        let result: Vec<i64> = decodable
            .array()
            .expect("failed to decode as array")
            .iter()
            .map(|d| {
                d.integer()
                    .expect("failed to decode array element as integer")
            })
            .collect();
        assert_eq!(result, values);
    }
}
