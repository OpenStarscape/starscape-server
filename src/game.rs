use cgmath::*;
use std::io::{self, Write};

use crate::body::Body;
use crate::connection::Connection;
use crate::physics::{apply_collisions, apply_gravity, apply_motion};
use crate::ship::create_ship;
use crate::state::State;

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
        let ship_a = create_ship(&mut game.state, Point3::new(0.0, 100000.0, 0.0));
        create_ship(&mut game.state, Point3::new(1.0, 0.0, 0.0));
        game.state.add_body(
            Body::new()
                .with_position(Point3::origin())
                .with_mass(1.0e+18)
                .with_gravity(),
        );
        let connection = game
            .state
            .connections
            .insert_with_key(|key| Connection::new(key, Box::new(io::stdout())));
        let connection_object_id = game.state.connections[connection].register_object(ship_a);
        game.state.connections[connection].subscribe_to(
            &game.state,
            connection_object_id,
            "position",
        );
        game
    }

    /// Runs a single iteration of the game loop
    /// Returns if to continue the game
    pub fn step(&mut self) -> bool {
        println!(" -- Game time: {}", self.state.time);

        apply_gravity(&mut self.state, self.step_dt);
        apply_collisions(&mut self.state, self.step_dt);
        apply_motion(&mut self.state, self.step_dt);

        {
            let mut updates = self
                .state
                .pending_updates
                .write()
                .expect("Failed to write to updates");
            for update in &*updates {
                if let Some(conduit) = self.state.conduits.get(*update) {
                    if let Err(e) = conduit.send_updates(&self.state) {
                        eprintln!("Error sending update: {}", e);
                    }
                }
            }
            updates.clear();
        }

        self.state.time += self.step_dt;
        if self.state.time > 10.0 {
            self.should_quit = true;
        }
        !self.should_quit
    }
}
