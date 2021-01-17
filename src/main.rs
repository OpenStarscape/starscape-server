//! This is the OpenStarscape game engine and server. OpenStarscape is an open source multiplayer
//! space flight simulator that encourages 3rd party clients. See `../hacking.md` for an
//! architecture overview and coding guidlines.

#[macro_use]
extern crate log;

#[macro_use(new_key_type)]
extern crate slotmap;

mod connection;
#[allow(clippy::new_ret_no_self)]
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
use slotmap::{DenseSlotMap, Key};
use weak_self::WeakSelf;

use std::error::Error;
use std::{
    any::{type_name, Any},
    collections::{HashMap, HashSet},
    f64::consts::PI,
    fmt::{Debug, Formatter},
    marker::PhantomData,
    ops::Deref,
    sync::mpsc::{channel, Receiver, Sender},
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc, Mutex, RwLock, Weak,
    },
};

/// The number of game ticks/second
const TICKS_PER_SEC: u32 = 15;
/// Used for both physics and the real timing of the game
const TICK_TIME: f64 = 1.0 / TICKS_PER_SEC as f64;
/// The amount of time the engine is given to do it's thing each tick. If it can't complete a tick
/// on time, the game will slow down.
const TIME_BUDGET: f64 = 0.01;
/// Clients that can complete a roundtrip faster than this will be able to respond before any
/// additional updates are made and will all be on a level playing field. The engine must
/// be able to complete a full tick in the gap between this and TICK_TIME. If it can't, the game
/// will be slowed down.
const MIN_SLEEP_TIME: f64 = TICK_TIME - TIME_BUDGET;

/// The total time (in in-game seconds) before the engine shuts down
const GAME_TIME: f64 = 10.0 * 60.0;

/// By default show error, warn and info messages
fn init_logger() {
    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
}

/// This gives us graceful shutdown when the user quits with Ctrl+C on the terminal
fn init_ctrlc_handler() -> Receiver<()> {
    let (tx, rx) = channel();
    ctrlc::set_handler(move || {
        warn!("processing Ctrl+C from user…");
        tx.send(()).expect("failed to send quit signal");
    })
    .expect("error setting Ctrl+C handler");
    rx
}

#[tokio::main]
async fn main() {
    init_logger();
    let ctrlc_rx = init_ctrlc_handler();

    info!("initializing game…");

    // Create a server, which will spin up everything required to talk to clients. The server object
    // is not used directly but needs to be kept in scope for as long as the game runs.
    let (new_session_tx, new_session_rx) = channel();
    let _server = Server::new(true, true, true, Some("../web/dist"), new_session_tx)
        .unwrap_or_else(|e| {
            error!("{}", e);
            panic!("failed to create game");
        });
    // Create the game engine. The `init` and `physics_tick` callbacks are the entiry points into
    // the `game` module
    let mut engine = Engine::new(
        new_session_rx,
        TICK_TIME,
        GAME_TIME,
        game::init,
        game::physics_tick,
    );

    info!("running game…");

    let mut metronome = Metronome::new(TICK_TIME, MIN_SLEEP_TIME);
    while engine.tick() {
        metronome.sleep();
        if ctrlc_rx.try_recv().is_ok() {
            trace!("exiting game loop due to quit signal");
            break;
        }
    }

    info!("game stopped")
}
