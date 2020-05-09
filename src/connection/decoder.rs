use std::error::Error;

use super::*;

/// Decodes a stream of bytes from the session into requests
pub trait Decoder {
    fn decode(&mut self, bytes: Vec<u8>) -> Result<Vec<Request>, Box<dyn Error>>;
}
