use std::error::Error;

use super::{ObjectId, Value};

pub trait Encoder {
    fn encode_property_update(
        &self,
        object: ObjectId,
        property: &str,
        value: &Value,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
}
