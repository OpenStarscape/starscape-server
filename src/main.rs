#[macro_use(new_key_type)]
extern crate slotmap;

mod body;
mod conduit;
mod connection;
mod entity;
mod game;
mod physics;
mod property;
mod serialize;
mod ship;
mod state;

pub const EPSILON: f64 = 0.000001;

fn main() {
    println!("Initializing game...");
    let mut game = game::Game::new();
    println!("Starting game...");
    while game.step() {}
    println!("Done")
}
