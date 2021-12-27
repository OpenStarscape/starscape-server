//! The lower level network code, including the session layer for supported network protocols

use super::*;

mod http;
mod ip_addrs;
mod server;
mod server_config;
mod session;
mod tcp;
mod webrtc;
mod websocket;

pub use server::Server;
pub use server_config::*;
pub use session::{InboundBundleHandler, Session, SessionBuilder};

use http::*;
use ip_addrs::*;
use server::ServerComponent;
use tcp::*;
use webrtc::*;
use websocket::*;

type GenericFilter = warp::filters::BoxedFilter<(Box<dyn warp::Reply>,)>;
use std::net::{IpAddr, SocketAddr};
use warp::Filter;
