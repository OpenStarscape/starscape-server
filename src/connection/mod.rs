mod connection_impl;
mod connection_trait;
mod encoder;
mod json_protocol;
mod object_map;
mod value;

pub use connection_trait::Connection;
pub use value::Value;

use connection_impl::ConnectionImpl;
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
