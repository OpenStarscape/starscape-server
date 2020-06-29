#[macro_use(new_key_type)]
extern crate slotmap;

mod entity;
mod game;
mod components;
mod god;
mod physics;
mod plumbing;
mod server;
mod state;
mod util;

pub use state::{EntityKey, State};

pub const EPSILON: f64 = 0.000_001;

fn main() {
    println!("Initializing game...");
    let mut game = game::Game::new();
    println!("Starting game...");
    while game.step() {}
    println!("Done")
}
