use cgmath::*;
use std::{
    sync::mpsc::{channel, Receiver},
    thread::sleep,
    time::Duration,
};

use crate::body::Body;
use crate::connection::new_json_connection;
use crate::god::create_god;
use crate::network::{Listener, SessionBuilder, TcpListener};
use crate::physics::{apply_collisions, apply_gravity, apply_motion};
use crate::ship::create_ship;
use crate::state::State;

const STEPS_PER_SEC: u64 = 30;

pub struct Game {
    should_quit: bool,
    /// In-game delta-time for each game step
    step_dt: f64,
    /// The entire game state
    state: State,
    servers: Vec<Box<dyn Listener>>,
    new_session_rx: Receiver<Box<dyn SessionBuilder>>,
}

impl Game {
    pub fn new() -> Game {
        let (new_session_tx, new_session_rx) = channel();
        let mut game = Game {
            should_quit: false,
            step_dt: 1.0 / STEPS_PER_SEC as f64,
            state: State::new(),
            servers: Vec::new(),
            new_session_rx,
        };
        let god = create_god(&mut game.state);
        let ship_a = create_ship(&mut game.state, Point3::new(0.0, 100_000.0, 0.0));
        create_ship(&mut game.state, Point3::new(1.0, 0.0, 0.0));
        game.state.add_body(
            Body::new()
                .with_position(Point3::origin())
                .with_mass(1.0e+18)
                .with_gravity(),
        );
        game.servers.push(Box::new(
            TcpListener::new(new_session_tx, None, None).expect("failed to create TCP server"),
        ));
        game
    }

    // TODO: abstract this out and test it
    fn try_build_connection(&mut self, builder: Box<dyn SessionBuilder>) {
        eprintln!("New session: {:?}", builder);
        // hack to get around slotmap only giving us a key after creation
        let key = self.state.connections.insert(Box::new(()));
        match new_json_connection(key, builder) {
            Ok(c) => {
                self.state.connections[key] = c;
            }
            Err(e) => {
                self.state.connections.remove(key);
                eprintln!("Error building connection: {}", e);
            }
        }
    }

    /// Runs a single iteration of the game loop
    /// Returns if to continue the game
    pub fn step(&mut self) -> bool {
        sleep(Duration::from_micros(1_000_000 / STEPS_PER_SEC));

        while let Ok(session_builder) = self.new_session_rx.try_recv() {
            self.try_build_connection(session_builder);
        }

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
                    if let Err(e) = conduit.send_updates(&self.state) {
                        eprintln!("Error sending update: {}", e);
                    }
                }
            }
            updates.clear();
        }

        self.state.time += self.step_dt;
        if self.state.time > 20.0 {
            self.should_quit = true;
        }
        !self.should_quit
    }
}
