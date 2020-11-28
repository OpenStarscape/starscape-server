use super::*;
use std::{thread::sleep, time::Duration};

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
    pub fn new() -> Result<Game, Box<dyn Error>> {
        let server = Box::new(ServerImpl::new(true, true)?);
        let mut game = Game {
            should_quit: false,
            step_dt: 1.0 / STEPS_PER_SEC as f64,
            state: State::new(),
            server,
        };
        let _god = create_god(&mut game.state);
        let _ship_a = create_ship(&mut game.state, Point3::new(0.0, 100_000.0, 0.0));
        create_ship(&mut game.state, Point3::new(1.0, 0.0, 0.0));
        let planet = game.state.create_entity();
        game.state.install_component(
            planet,
            Body::new()
                .with_position(Point3::origin())
                .with_mass(1.0e+18),
        );
        game.state.install_component(planet, GravityBody());
        Ok(game)
    }

    /// Runs a single iteration of the game loop
    /// Returns if to continue the game
    pub fn step(&mut self) -> bool {
        sleep(Duration::from_micros(1_000_000 / STEPS_PER_SEC));

        self.server.process_requests(&mut self.state);

        apply_gravity(&mut self.state, self.step_dt);
        apply_collisions(&self.state, self.step_dt);
        apply_motion(&mut self.state, self.step_dt);

        let notifications = std::mem::take(&mut self.state.pending_updates);
        for notification in notifications {
            if let Some(sink) = notification.upgrade() {
                if let Err(e) = sink.notify(&self.state, self.server.property_update_sink()) {
                    error!("failed to process notification: {}", e);
                }
            }
        }

        self.state.time += self.step_dt;
        !self.should_quit
    }
}
