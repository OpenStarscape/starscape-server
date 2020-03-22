pub struct Game {
    should_quit: bool,
    steps: i64,
}

impl Game {
    pub fn new() -> Game {
        Game {
            should_quit: false,
            steps: 0,
        }
    }

    /// Runs a single iteration of the game loop
    /// Returns if to continue the game
    pub fn step(&mut self) -> bool {
        println!("Game loop step {}", self.steps);

        // TODO: Process incoming messages
        // TODO: Apply gravity and thrust to objects' deltas
        // TODO: Apply objects' deltas to their positions
        // TODO: Check for collisions
        // TODO: Send updates
        // TODO: Wait until it's time for another cycle

        self.steps += 1;
        if self.steps > 100 {
            self.should_quit = true;
        }
        !self.should_quit
    }
}
