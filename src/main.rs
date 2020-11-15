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

fn main() {
    println!("Initializing game...");
    let (quit_sender, quit_receiver) = channel();
    ctrlc::set_handler(move || {
        println!("Processing Ctrl+C from user...");
        quit_sender.send(()).expect("failed to send quit signal");
    })
    .expect("Error setting Ctrl-C handler");
    let mut game = Game::new();
    println!("Running game...");
    while game.step() {
        if quit_receiver.try_recv().is_ok() {
            println!("Exiting game loop due to quit signal");
            break;
        }
    }
    println!("Done")
}
