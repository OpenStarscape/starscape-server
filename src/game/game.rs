use super::*;
use std::{thread::sleep, time::Duration};

const STEPS_PER_SEC: u64 = 30;

pub struct Game {
    should_quit: bool,
    /// In-game delta-time for each physics step
    step_dt: f64,
    state: State,
    connections: ConnectionCollection,
    _server: Server,
}

impl Game {
    pub fn new() -> Result<Game, Box<dyn Error>> {
        let mut state = State::new();
        let god = create_god(&mut state);
        let connections = ConnectionCollection::new(god);
        let server = Server::new(true, true, connections.session_sender())?;
        let mut game = Game {
            should_quit: false,
            step_dt: 1.0 / STEPS_PER_SEC as f64,
            state,
            connections,
            _server: server,
        };
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

        self.connections.process_inbound_messages(&mut self.state);

        apply_gravity(&mut self.state, self.step_dt);
        apply_collisions(&self.state, self.step_dt);
        apply_motion(&mut self.state, self.step_dt);

        let notifications = std::mem::take(&mut self.state.pending_updates);
        for notification in notifications {
            if let Some(sink) = notification.upgrade() {
                if let Err(e) = sink.notify(&self.state, &self.connections) {
                    error!("failed to process notification: {}", e);
                }
            }
        }

        self.connections.flush_outbound_messages(&mut self.state);

        self.state.time += self.step_dt;
        !self.should_quit
    }
}
