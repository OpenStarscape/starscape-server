use super::*;

mod http;
mod listener;
mod request_handler;
#[allow(clippy::module_inception)]
mod server_impl;
mod server_trait;
mod session;
mod tcp;
mod webrtc;

pub use request_handler::RequestHandler;
pub use server_impl::{ConnectionKey, ServerImpl};
pub use server_trait::{PropertyUpdateSink, Server};
pub use session::{Session, SessionBuilder};

use http::*;
use listener::Listener;
use tcp::*;
use webrtc::*;

type GenericFilter = warp::filters::BoxedFilter<(Box<dyn warp::Reply>,)>;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use warp::Filter;
