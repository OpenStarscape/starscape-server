mod connection_impl;
mod connection_trait;
mod request;

pub use connection_trait::Connection;
pub use request::{ObjectProperty, Request};

use super::*;
use connection_impl::ConnectionImpl;

use crate::state::EntityKey;
use std::error::Error;

pub fn new_json_connection(
    self_key: crate::state::ConnectionKey,
    session_builder: Box<dyn SessionBuilder>,
) -> Result<Box<dyn Connection>, Box<dyn Error>> {
    let (encoder, decoder) = json_protocol_impls();
    let conn = ConnectionImpl::new(self_key, encoder, decoder, session_builder)?;
    Ok(Box::new(conn))
}
