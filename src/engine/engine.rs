use super::*;

pub struct Engine {
    should_quit: bool,
    quit_after: f64,
    /// In-game delta-time for each physics step
    physics_tick_delta: f64,
    pub state: State,
    back_notif_buffer: Vec<Notification>,
    connections: ConnectionCollection,
    physics_tick: Box<dyn Fn(&mut State, f64)>,
}

impl Engine {
    pub fn new<InitFn, TickFn>(
        new_session_rx: Receiver<Box<dyn SessionBuilder>>,
        physics_tick_delta: f64,
        quit_after: f64,
        init: InitFn,
        physics_tick: TickFn,
    ) -> Self
    where
        InitFn: Fn(&mut State),
        TickFn: Fn(&mut State, f64) + 'static,
    {
        let mut state = State::new();
        let connections = ConnectionCollection::new(new_session_rx, state.root_entity(), 10);
        init(&mut state);
        Self {
            should_quit: false,
            quit_after,
            physics_tick_delta,
            state,
            back_notif_buffer: Vec::new(),
            connections,
            physics_tick: Box::new(physics_tick),
        }
    }

    /// Runs a single iteration of the game loop
    /// Returns if to continue the game
    pub fn tick(&mut self) -> bool {
        self.connections.process_inbound_messages(&mut self.state);

        (self.physics_tick)(&mut self.state, self.physics_tick_delta);

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

        self.state.time += self.physics_tick_delta;
        if self.state.time > self.quit_after {
            self.should_quit = true;
            info!(
                "engine has run for {:?}, stopping…",
                std::time::Duration::from_secs_f64(self.quit_after)
            )
        }
        !self.should_quit
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.connections.finalize(&mut self.state);
    }
}
