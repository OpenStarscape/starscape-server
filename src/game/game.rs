use super::*;
use std::{thread::sleep, time::Duration};

const STEPS_PER_SEC: u64 = 30;

pub struct Game {
    should_quit: bool,
    /// In-game delta-time for each physics step
    step_dt: f64,
    state: State,
    back_notif_buffer: Vec<Notification>,
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
            back_notif_buffer: Vec::new(),
            connections,
            _server: server,
        };
        let _ship_a = create_ship(
            &mut game.state,
            Point3::new(100_000.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 5_000.0),
        );
        let _ship_b = create_ship(
            &mut game.state,
            Point3::new(0.0, 0.0, 60_000.0),
            Vector3::new(10_000.0, 1_000.0, 4_000.0),
        );
        let planet = game.state.create_entity();
        Body::new()
            .with_class(BodyClass::Celestial)
            .with_position(Point3::origin())
            .with_sphere_shape(6_000.0)
            .with_mass(1.0e+15)
            .install(&mut game.state, planet);
        let moon = game.state.create_entity();
        Body::new()
            .with_class(BodyClass::Celestial)
            .with_position(Point3::new(60_000.0, 0.0, 0.0))
            .with_velocity(Vector3::new(0.0, 0.0, 10_000.0))
            .with_sphere_shape(2_000.0)
            .with_mass(1.0e+14)
            .install(&mut game.state, moon);
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

        self.state
            .notif_queue
            .swap_buffer(&mut self.back_notif_buffer);
        for notification in &self.back_notif_buffer {
            if let Some(sink) = notification.upgrade() {
                if let Err(e) = sink.notify(&self.state, &self.connections) {
                    error!("failed to process notification: {}", e);
                }
            }
        }
        // this does not deallocate, so we don't need to reallocate every cycle
        self.back_notif_buffer.clear();

        self.connections.flush_outbound_messages(&mut self.state);

        self.state.time += self.step_dt;
        !self.should_quit
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        self.connections.finalize(&mut self.state);
    }
}
