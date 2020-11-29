//! All things related to decoding (deserializing) data from the client

use super::*;

/// A value received from a client which can be decoded into a value. Prefer the accessor methods or
/// simply .decode() rather than matching directly
#[derive(Debug, PartialEq, Clone)]
pub enum Decodable {
    Scaler(f64),
    Integer(i64),
    List(Vec<Decodable>),
    Null,
}

/// The context required for decoding a Decodable
pub trait DecodeCtx: Send + Sync {
    /// Returns the entity for the given object ID, or Err if it does not exist
    fn entity_for(&self, object: ObjectId) -> Result<EntityKey, Box<dyn Error>>;
}

pub trait DecodableAs<T> {
    fn decode(&self) -> Result<T, String>;
}

pub type DecodableResult<T> = Result<T, String>;

impl Decodable {
    pub fn vector(&self) -> DecodableResult<Vector3<f64>> {
        match self {
            Decodable::List(list) if list.len() == 3 => Ok(Vector3::new(
                list[0].scaler()?,
                list[1].scaler()?,
                list[2].scaler()?,
            )),
            _ => Err(format!("{:?} is not a 3D vector", self)),
        }
    }

    pub fn scaler(&self) -> DecodableResult<f64> {
        match self {
            Decodable::Scaler(value) => Ok(*value),
            Decodable::Integer(value) => Ok(*value as f64),
            _ => Err(format!("{:?} is not a number", self)),
        }
    }

    pub fn integer(&self) -> DecodableResult<i64> {
        match self {
            Decodable::Scaler(value) => Ok(*value as i64),
            Decodable::Integer(value) => Ok(*value),
            _ => Err(format!("{:?} is not a number", self)),
        }
    }

    pub fn entity(&self) -> DecodableResult<EntityKey> {
        // TODO: include an option object map Arc with the integer
        Err(format!("{:?} is not an object", self))
    }

    #[allow(dead_code)]
    pub fn list<'a>(&'a self) -> DecodableResult<Box<dyn Iterator<Item = &Decodable> + 'a>> {
        match self {
            Decodable::List(list) => Ok(Box::new(list.iter())),
            _ => Err(format!("{:?} is not a list", self)),
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Decodable::Null => true,
            _ => false,
        }
    }
}

impl DecodableAs<Vector3<f64>> for Decodable {
    fn decode(&self) -> Result<Vector3<f64>, String> {
        self.vector()
    }
}

impl DecodableAs<Point3<f64>> for Decodable {
    fn decode(&self) -> Result<Point3<f64>, String> {
        self.vector().map(Point3::from_vec)
    }
}

impl DecodableAs<f64> for Decodable {
    fn decode(&self) -> Result<f64, String> {
        self.scaler()
    }
}

impl DecodableAs<i64> for Decodable {
    fn decode(&self) -> Result<i64, String> {
        self.integer()
    }
}

impl DecodableAs<u64> for Decodable {
    fn decode(&self) -> Result<u64, String> {
        match self.integer().map(std::convert::TryFrom::try_from) {
            Ok(Ok(i)) => Ok(i),
            _ => Err(format!("{:?} is not an unsigned integer", self)),
        }
    }
}

impl DecodableAs<EntityKey> for Decodable {
    fn decode(&self) -> Result<EntityKey, String> {
        match self.entity() {
            Ok(e) => Ok(e),
            _ => {
                if let Ok(i) = self.decode() {
                    let i: u64 = i;
                    Err(format!("object {} does not exist", i))
                } else {
                    Err(format!("{:?} is not an object ID", self))
                }
            }
        }
    }
}

impl DecodableAs<()> for Decodable {
    fn decode(&self) -> Result<(), String> {
        if self.is_null() {
            Ok(())
        } else {
            Err(format!("{:?} is not null", self))
        }
    }
}

/// Decodes a stream of bytes from the session into requests
pub trait Decoder: Send {
    fn decode(
        &mut self,
        ctx: &dyn DecodeCtx,
        bytes: Vec<u8>,
    ) -> Result<Vec<RequestType>, Box<dyn Error>>;
}

#[cfg(test)]
mod json_tests {
    use super::*;
    use Decodable::*;

    fn assert_decodes_to<T>(decodable: &Decodable, expected: T)
    where
        T: PartialEq + Debug,
        Decodable: DecodableAs<T>,
    {
        let actual: T = decodable.decode().expect("failed to decode");
        assert_eq!(actual, expected);
    }

    #[test]
    fn numbers_decode_correctly() {
        let i: i64 = 7;
        let u: u64 = 7;
        let f: f64 = 7.0;
        assert_decodes_to(&Integer(i), i);
        assert_decodes_to(&Integer(i), f);
        assert_decodes_to(&Integer(i), u);
        assert_decodes_to(&Scaler(f), i);
        assert_decodes_to(&Scaler(f), f);
        assert_decodes_to(&Scaler(f), u);
    }

    #[test]
    fn vectors_and_points_decode_correctly() {
        let point = Point3::new(1.0, 2.5, -3.0);
        let vector = point.to_vec();
        let decodable = List(vec![Scaler(point.x), Scaler(point.y), Scaler(point.z)]);
        let mismatched_typed_decodable = List(vec![Integer(1), Scaler(point.y), Scaler(point.z)]);
        assert_decodes_to(&decodable, point);
        assert_decodes_to(&decodable, vector);
        assert_decodes_to(&mismatched_typed_decodable, vector);
    }

    #[test]
    fn can_decode_vec_of_ints() {
        let values = vec![7, 8, 9];
        let decodable = List(values.iter().map(|i| Integer(*i)).collect());
        let result: Vec<i64> = decodable
            .list()
            .expect("failed to decode as list")
            .map(|d| {
                d.integer()
                    .expect("failed to decode list element as integer")
            })
            .collect();
        assert_eq!(result, values);
    }
}
