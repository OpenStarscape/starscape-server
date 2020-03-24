#[macro_use(new_key_type)]
extern crate slotmap;

mod body;
mod game;
mod object;
mod ship;
mod state;

fn main() {
    println!("Initializing game...");
    let mut game = game::Game::new();
    println!("Starting game...");
    while game.step() {}
    println!("Done")
}
