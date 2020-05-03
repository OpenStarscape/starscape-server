use std::error::Error;

use super::*;

pub trait Encoder {
    fn encode_property_update(
        &self,
        object: ObjectId,
        property: &str,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
}
