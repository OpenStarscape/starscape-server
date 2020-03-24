use cgmath::Point3;

use super::ship::new_ship;
use super::state::State;

pub struct Game {
    should_quit: bool,
    time: f64,
    step_time: f64,
    state: State,
}

impl Game {
    pub fn new() -> Game {
        let mut game = Game {
            should_quit: false,
            time: 0.0,
            step_time: 1.0 / 60.0,
            state: State::new(),
        };
        new_ship(&mut game.state, Point3::new(0.0, 1.0, 0.0));
        game
    }

    /// Runs a single iteration of the game loop
    /// Returns if to continue the game
    pub fn step(&mut self) -> bool {
        println!("Game time: {}", self.time);

        for (_, body) in &self.state.bodies {
            body.step(&self.state, self.time, self.step_time, &vec![]);
        }

        // TODO: Process incoming messages
        // TODO: Apply gravity and thrust to objects' deltas
        //          - gravity sources
        //          - mut object deltas
        // TODO: Check for collisions
        //          - things
        //          - mut object deltas
        //          - mut object health?
        // TODO: Apply objects' deltas to their positions
        //          - object deltas
        //          - mut positions
        // TODO: Send updates
        // TODO: Wait until it's time for another cycle

        self.time += self.step_time;
        if self.time > 10.0 {
            self.should_quit = true;
        }
        !self.should_quit
    }
}
