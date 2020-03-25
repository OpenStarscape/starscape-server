#[macro_use(new_key_type)]
extern crate slotmap;

mod game;
mod state;
mod body;
mod ship;

fn main() {
    println!("Initializing game...");
    let mut game = game::Game::new();
    println!("Starting game...");
    while game.step() {}
    println!("Done")
}
