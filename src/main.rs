#[macro_use(new_key_type)]
extern crate slotmap;

mod body;
mod connection;
mod entity;
mod game;
mod god;
mod network;
mod physics;
mod plumbing;
mod ship;
mod state;
mod util;

pub use state::EntityKey;

pub const EPSILON: f64 = 0.000_001;

fn main() {
    println!("Initializing game...");
    let mut game = game::Game::new();
    println!("Starting game...");
    while game.step() {}
    println!("Done")
}
