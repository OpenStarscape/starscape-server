use cgmath::*;
use std::io;
use std::sync::mpsc::{channel, Receiver};

use crate::body::Body;
use crate::connection::new_json_connection;
use crate::god::create_god;
use crate::network::{Server, SessionBuilder, TcpServer};
use crate::physics::{apply_collisions, apply_gravity, apply_motion};
use crate::ship::create_ship;
use crate::state::State;

pub struct Game {
    should_quit: bool,
    /// Delta-time for each game step
    step_dt: f64,
    /// The entire game state
    state: State,
    servers: Vec<Box<dyn Server>>,
    new_session_rx: Receiver<Box<dyn SessionBuilder>>,
}

impl Game {
    pub fn new() -> Game {
        let (new_session_tx, new_session_rx) = channel();
        let mut game = Game {
            should_quit: false,
            step_dt: 1.0 / 60.0,
            state: State::new(),
            servers: Vec::new(),
            new_session_rx,
        };
        let connection = game
            .state
            .connections
            .insert_with_key(|key| new_json_connection(key, Box::new(io::stdout())));
        let god = create_god(&mut game.state);
        game.state.connections[connection].subscribe_to(&game.state, god, "bodies");
        let ship_a = create_ship(&mut game.state, Point3::new(0.0, 100_000.0, 0.0));
        create_ship(&mut game.state, Point3::new(1.0, 0.0, 0.0));
        game.state.add_body(
            Body::new()
                .with_position(Point3::origin())
                .with_mass(1.0e+18)
                .with_gravity(),
        );
        game.state.connections[connection].subscribe_to(&game.state, ship_a, "position");
        game.servers.push(Box::new(
            TcpServer::new(new_session_tx, None, None).expect("failed to create TCP server"),
        ));
        game
    }

    /// Runs a single iteration of the game loop
    /// Returns if to continue the game
    pub fn step(&mut self) -> bool {
        println!(" -- Game time: {}", self.state.time);

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
        if self.state.time > 10.0 {
            self.should_quit = true;
        }
        !self.should_quit
    }
}
