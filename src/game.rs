use cgmath::*;
use std::{thread::sleep, time::Duration};

use crate::god::create_god;
use crate::physics::{apply_collisions, apply_gravity, apply_motion};
use crate::server::Server;
use crate::components::{create_ship, Body};
use crate::state::State;

const STEPS_PER_SEC: u64 = 30;

pub struct Game {
    should_quit: bool,
    /// In-game delta-time for each game step
    step_dt: f64,
    /// The entire game state
    state: State,
    server: Box<dyn Server>,
}

impl Game {
    pub fn new() -> Game {
        let mut game = Game {
            should_quit: false,
            step_dt: 1.0 / STEPS_PER_SEC as f64,
            state: State::new(),
            server: Server::new_impl(true),
        };
        let _god = create_god(&mut game.state);
        let _ship_a = create_ship(&mut game.state, Point3::new(0.0, 100_000.0, 0.0));
        create_ship(&mut game.state, Point3::new(1.0, 0.0, 0.0));
        game.state.add_body(
            Body::new()
                .with_position(Point3::origin())
                .with_mass(1.0e+18)
                .with_gravity(),
        );
        game
    }

    /// Runs a single iteration of the game loop
    /// Returns if to continue the game
    pub fn step(&mut self) -> bool {
        sleep(Duration::from_micros(1_000_000 / STEPS_PER_SEC));

        self.server.process_requests(&mut self.state);

        apply_gravity(&mut self.state, self.step_dt);
        apply_collisions(&self.state, self.step_dt);
        apply_motion(&mut self.state, self.step_dt);

        {
            let mut updates = self
                .state
                .pending_updates
                .write()
                .expect("Failed to write to updates");
            for update in &*updates {
                if let Some(conduit) = self.state.properties.get(*update) {
                    if let Err(e) =
                        conduit.send_updates(&self.state, self.server.property_update_sink())
                    {
                        eprintln!("Error sending update: {}", e);
                    }
                }
            }
            updates.clear();
        }

        self.state.time += self.step_dt;
        !self.should_quit
    }
}
