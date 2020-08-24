#[macro_use(new_key_type)]
extern crate slotmap;

mod components;
mod entity;
mod game;
mod plumbing;
mod server;
mod state;
mod systems;
mod util;

use components::*;
use entity::*;
use game::*;
use plumbing::*;
use server::*;
use state::*;
use systems::*;
use util::*;

use cgmath::*;
use slotmap::DenseSlotMap;

use std::error::Error;
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    sync::{Arc, Mutex, RwLock, Weak},
};

pub const EPSILON: f64 = 0.000_001;

fn main() {
    println!("Initializing game...");
    let mut game = Game::new();
    println!("Starting game...");
    while game.step() {}
    println!("Done")
}
