//! # WebRTC session layer implementation
//!
//! WebRTC unreliable data channels are the only way to get data to and from
//! in-browser apps without head-of-line blocking. Implementing a WebRTC
//! server is quite difficult. We use the the
//! [webrtc-unreliable](https://crates.io/crates/webrtc-unreliable) crate wich
//! supports the bare-minimum features to enbable unreliable data channels.

use super::*;

mod webrtc_http_server;
mod webrtc_listener;
mod webrtc_session;

pub use webrtc_listener::WebrtcListener;

use webrtc_http_server::WebrtcHttpServer;
use webrtc_session::WebrtcSessionBuilder;
