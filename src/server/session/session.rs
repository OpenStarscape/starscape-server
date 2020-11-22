use super::*;

use std::fmt::Debug;

pub trait SessionBuilder: Send + Debug {
    fn build(
        self: Box<Self>,
        handle_incoming_data: Box<dyn FnMut(&[u8]) + Send>,
    ) -> Result<Box<dyn Session>, Box<dyn Error>>;
}

pub trait Session: Send + Debug {
    fn send(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>>;
}
