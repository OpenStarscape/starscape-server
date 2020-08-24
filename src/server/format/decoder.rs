use super::*;

/// Decodes a stream of bytes from the session into requests
pub trait Decoder: Send {
    fn decode(&mut self, bytes: Vec<u8>) -> Result<Vec<ConnectionRequest>, Box<dyn Error>>;
}
