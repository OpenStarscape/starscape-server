#[macro_use]
extern crate log;

#[macro_use(new_key_type)]
extern crate slotmap;

mod components;
mod engine;
mod game;
mod server;
mod systems;
mod util;

use components::*;
use engine::*;
use game::*;
use server::*;
use systems::*;
use util::*;

use anymap::AnyMap;
use cgmath::*;
use futures::{executor::block_on, future};
use slotmap::DenseSlotMap;

use std::error::Error;
use std::{
    any::type_name,
    collections::HashMap,
    fmt::{Debug, Formatter},
    marker::PhantomData,
    sync::mpsc::{channel, Receiver, Sender},
    sync::{Arc, Mutex, RwLock, Weak},
};

pub const EPSILON: f64 = 0.000_001;

#[tokio::main]
async fn main() {
    // By default show error, warn and info
    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
    info!("initializing game…");

    let (quit_sender, quit_receiver) = channel();
    ctrlc::set_handler(move || {
        warn!("processing Ctrl+C from user…");
        quit_sender.send(()).expect("failed to send quit signal");
    })
    .expect("error setting Ctrl-C handler");

    let mut game = Game::new();

    info!("running game…");
    while game.step() {
        if quit_receiver.try_recv().is_ok() {
            trace!("exiting game loop due to quit signal");
            break;
        }
    }

    info!("game stopped")
}
