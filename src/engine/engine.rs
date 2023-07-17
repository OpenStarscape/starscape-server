use super::*;

pub struct Engine {
    pub state: State,
    back_notif_buffer: Vec<Notification>,
    connections: ConnectionCollection,
    game_tick: Box<dyn Fn(&mut State) -> bool>,
}

impl Engine {
    pub fn new<InitFn, TickFn>(
        game_config: &GameConfig,
        trace_level: TraceLevel,
        new_session_rx: Receiver<Box<dyn SessionBuilder>>,
        init: InitFn,
        game_tick: TickFn,
    ) -> Self
    where
        InitFn: Fn(&mut State, &GameConfig),
        TickFn: Fn(&mut State) -> bool + 'static,
    {
        let mut state = State::new();
        let connections = ConnectionCollection::new(new_session_rx, state.root(), 10, trace_level);
        init(&mut state, game_config);
        Self {
            state,
            back_notif_buffer: Vec::new(),
            connections,
            game_tick: Box::new(game_tick),
        }
    }

    /// Runs a single iteration of the game loop
    /// Returns if to continue the game
    pub fn tick(&mut self) -> bool {
        self.connections.process_inbound_messages(&mut self.state);

        let should_quit = (self.game_tick)(&mut self.state);

        self.state
            .notif_queue
            .swap_buffer(&mut self.back_notif_buffer);
        for notification in &self.back_notif_buffer {
            if let Some(notif) = notification.upgrade() {
                notif.notify(&self.state, &self.connections);
            }
        }
        // this does not deallocate, so we don't need to reallocate every cycle
        self.back_notif_buffer.clear();

        self.connections.flush_outbound_messages(&mut self.state);

        if !should_quit {
            self.state.metronome.sleep();
        }
        !should_quit
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.connections.finalize(&mut self.state);
    }
}
