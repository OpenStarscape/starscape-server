use cgmath::Point3;

use super::physics::{apply_gravity, apply_motion};
use super::ship::new_ship;
use super::state::State;

pub struct Game {
    should_quit: bool,
    /// Delta-time for each game step
    step_dt: f64,
    /// The entire game state
    state: State,
}

impl Game {
    pub fn new() -> Game {
        let mut game = Game {
            should_quit: false,
            step_dt: 1.0 / 60.0,
            state: State::new(),
        };
        new_ship(&mut game.state, Point3::new(0.0, 1.0, 0.0));
        game
    }

    /// Runs a single iteration of the game loop
    /// Returns if to continue the game
    pub fn step(&mut self) -> bool {
        println!("Game time: {}", self.state.time);

        apply_gravity(&mut self.state, self.step_dt);
        apply_motion(&mut self.state, self.step_dt);

        self.state.time += self.step_dt;
        if self.state.time > 10.0 {
            self.should_quit = true;
        }
        !self.should_quit
    }
}
