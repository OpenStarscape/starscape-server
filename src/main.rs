//! This is the OpenStarscape game engine and server. OpenStarscape is an open source multiplayer
//! space flight simulator that encourages 3rd party clients. See `../hacking.md` for an
//! architecture overview and coding guidlines.

#[macro_use]
extern crate log;

#[macro_use(new_key_type)]
extern crate slotmap;

mod connection;
mod engine;
#[allow(clippy::unit_arg)]
mod game;
mod helpers;
mod server;

use connection::*;
use engine::*;
use helpers::*;
use server::*;

use anymap::AnyMap;
use cgmath::*;
use futures::{executor::block_on, future, StreamExt};
use slotmap::DenseSlotMap;
use weak_self::WeakSelf;

use std::error::Error;
use std::{
    any::type_name,
    collections::{HashMap, HashSet},
    fmt::{Debug, Formatter},
    marker::PhantomData,
    ops::Deref,
    sync::mpsc::{channel, Receiver, Sender},
    sync::{Arc, Mutex, RwLock, Weak},
};

const PHYSICS_TICKS_PER_SEC: u64 = 30;

#[tokio::main]
async fn main() {
    // By default show error, warn and info
    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
    info!("initializing game…");

    // This gives us graceful shutdown when the user quits with Ctrl+C on the terminal
    let (quit_sender, quit_receiver) = channel();
    ctrlc::set_handler(move || {
        warn!("processing Ctrl+C from user…");
        quit_sender.send(()).expect("failed to send quit signal");
    })
    .expect("error setting Ctrl-C handler");

    // Create a server, which will spin up everything required to talk to clients
    let (new_session_tx, new_session_rx) = channel();
    let _server = Server::new(true, true, new_session_tx).unwrap_or_else(|e| {
        error!("{}", e);
        panic!("failed to create game");
    });

    // Create the game engine. The `init` and `physics_tick` callbacks are the entiry points into
    // the `game` module
    let delta = 1.0 / PHYSICS_TICKS_PER_SEC as f64;
    let mut engine = Engine::new(new_session_rx, delta, game::init, game::physics_tick);

    info!("running game…");
    while engine.tick() {
        std::thread::sleep(std::time::Duration::from_micros(
            (1_000_000.0 * delta) as u64,
        ));
        if quit_receiver.try_recv().is_ok() {
            trace!("exiting game loop due to quit signal");
            break;
        }
    }

    info!("game stopped")
}
