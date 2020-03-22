mod game;

fn main() {
    println!("Initializing game...");
    let mut game = game::Game::new();
    println!("Starting game...");
    while game.step() {}
    println!("Done")
}
