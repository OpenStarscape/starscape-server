mod connection_impl;
mod connection_trait;
mod datagram_splitter;
mod decodable;
mod encodable;
mod encoder;
mod json_protocol;
mod object_map;

pub use connection_trait::Connection;
pub use decodable::Decodable;
pub use encodable::Encodable;

use connection_impl::ConnectionImpl;
use datagram_splitter::DatagramSplitter;
use encoder::*;
use json_protocol::JsonProtocol;
use object_map::{ObjectId, ObjectMap};

pub fn new_json_connection(
    self_key: crate::state::ConnectionKey,
    writer: Box<dyn std::io::Write>,
) -> Box<dyn Connection> {
    Box::new(ConnectionImpl::new(
        self_key,
        Box::new(JsonProtocol::new()),
        writer,
    ))
}
