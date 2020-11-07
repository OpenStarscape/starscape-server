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
