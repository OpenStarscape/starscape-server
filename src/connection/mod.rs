mod connection_impl;
mod connection_trait;
mod datagram_splitter;
mod decodable;
mod decoder;
mod encodable;
mod encoder;
mod json_protocol;
mod object_map;
mod request;

pub use connection_trait::Connection;
pub use decodable::Decodable;
pub use encodable::Encodable;

use connection_impl::ConnectionImpl;
use datagram_splitter::DatagramSplitter;
use decoder::Decoder;
use encoder::*;
use json_protocol::*;
use object_map::{ObjectId, ObjectMap};
use request::*;

use crate::network::SessionBuilder;
use crate::state::EntityKey;
use std::error::Error;

pub fn new_json_connection(
    self_key: crate::state::ConnectionKey,
    session_builder: Box<dyn SessionBuilder>,
) -> Result<Box<dyn Connection>, Box<dyn Error>> {
    let conn = ConnectionImpl::new(
        self_key,
        Box::new(JsonEncoder::new()),
        Box::new(JsonDecoder::new()),
        session_builder,
    )?;
    Ok(Box::new(conn))
}
