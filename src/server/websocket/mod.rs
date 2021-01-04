//! # WebSocket session layer implementation
//!
//! WebSockets allow for TCP-like communication between a web frontend and the server. Like TCP
//! websockets have head-of-line blocking (a disadvantage compared to WebRTC), but they are simpler
//! than WebRTC and may be less buggy.

use super::*;

mod websocket_server;
mod websocket_session;
mod websocket_warp_filter;

pub use websocket_server::WebsocketServer;

use websocket_session::WebsocketSessionBuilder;
use websocket_warp_filter::websocket_warp_filter;
