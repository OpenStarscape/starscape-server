use cgmath::*;
use std::fmt::Debug;

use crate::state::EntityKey;

pub trait DecodableAs<T> {
    fn decode(&self) -> Result<T, String>;
}

pub type DecodableResult<T> = Result<T, String>;

pub trait Decodable: Debug {
    fn vector(&self) -> DecodableResult<Vector3<f64>> {
        Err(format!("{:?} is not a 3D vector", self))
    }
    fn scaler(&self) -> DecodableResult<f64> {
        Err(format!("{:?} is not a scaler", self))
    }
    fn integer(&self) -> DecodableResult<i64> {
        Err(format!("{:?} is not an integer", self))
    }
    fn entity(&self) -> DecodableResult<EntityKey> {
        Err(format!("{:?} is not an object", self))
    }
    fn list<'a>(&'a self) -> DecodableResult<Box<dyn Iterator<Item = &'a dyn Decodable> + 'a>> {
        Err(format!("{:?} is not a list", self))
    }
    fn is_null(&self) -> bool {
        false
    }
}

impl Decodable for Vector3<f64> {
    fn vector(&self) -> DecodableResult<Vector3<f64>> {
        Ok(*self)
    }
}

impl Decodable for Point3<f64> {
    fn vector(&self) -> DecodableResult<Vector3<f64>> {
        Ok(self.to_vec())
    }
}

impl Decodable for f64 {
    fn scaler(&self) -> DecodableResult<f64> {
        Ok(*self)
    }
    fn integer(&self) -> DecodableResult<i64> {
        Ok(self.round() as i64)
    }
}

impl Decodable for i64 {
    fn scaler(&self) -> DecodableResult<f64> {
        Ok(*self as f64)
    }
    fn integer(&self) -> DecodableResult<i64> {
        Ok(*self)
    }
}

impl Decodable for Vec<Box<dyn Decodable>> {
    fn list<'a>(&'a self) -> DecodableResult<Box<dyn Iterator<Item = &'a dyn Decodable> + 'a>> {
        Ok(Box::new(self.iter().map(|d| &**d)))
    }
}

impl Decodable for () {
    fn is_null(&self) -> bool {
        true
    }
}

impl<'a> DecodableAs<Vector3<f64>> for dyn Decodable + 'a {
    fn decode(&self) -> Result<Vector3<f64>, String> {
        self.vector()
    }
}

impl<'a> DecodableAs<Point3<f64>> for dyn Decodable + 'a {
    fn decode(&self) -> Result<Point3<f64>, String> {
        self.vector().map(Point3::from_vec)
    }
}

impl<'a> DecodableAs<f64> for dyn Decodable + 'a {
    fn decode(&self) -> Result<f64, String> {
        self.scaler()
    }
}

impl<'a> DecodableAs<i64> for dyn Decodable + 'a {
    fn decode(&self) -> Result<i64, String> {
        self.integer()
    }
}

impl<'a> DecodableAs<u64> for dyn Decodable + 'a {
    fn decode(&self) -> Result<u64, String> {
        match self.integer().map(std::convert::TryFrom::try_from) {
            Ok(Ok(i)) => Ok(i),
            _ => Err(format!("{:?} is not an unsigned integer", self)),
        }
    }
}

impl<'a> DecodableAs<EntityKey> for dyn Decodable + 'a {
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

impl<'a> DecodableAs<()> for dyn Decodable + 'a {
    fn decode(&self) -> Result<(), String> {
        if self.is_null() {
            Ok(())
        } else {
            Err(format!("{:?} is not null", self))
        }
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;

    fn assert_decodes_to<'a, T>(decodable: &'a dyn Decodable, expected: T)
    where
        T: PartialEq + Debug,
        dyn Decodable + 'a: DecodableAs<T>,
    {
        let actual: T = decodable.decode().expect("failed to decode");
        assert_eq!(actual, expected);
    }

    #[test]
    fn numbers_decode_correctly() {
        let i: i64 = 7;
        let u: u64 = 7;
        let f: f64 = 7.0;
        assert_decodes_to(&i, i);
        assert_decodes_to(&i, f);
        assert_decodes_to(&i, u);
        assert_decodes_to(&f, i);
        assert_decodes_to(&f, f);
        assert_decodes_to(&f, u);
    }

    #[test]
    fn vectors_and_points_decode_correctly() {
        let point = Point3::new(1.0, 2.0, -3.0);
        let vector = Vector3::new(1.0, 2.0, -3.0);
        assert_decodes_to(&point, point);
        assert_decodes_to(&vector, point);
        assert_decodes_to(&point, vector);
        assert_decodes_to(&vector, vector);
    }

    #[test]
    fn can_decode_vec_of_ints() {
        let values = vec![7, 8, 9];
        let decodables: Vec<Box<dyn Decodable>> = values
            .iter()
            .map(|i| Box::new(*i) as Box<dyn Decodable>)
            .collect();
        let decodable: Box<dyn Decodable> = Box::new(decodables);
        let result: Vec<i64> = (*decodable)
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
