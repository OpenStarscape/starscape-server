use super::*;

pub struct Engine {
    should_quit: bool,
    quit_after: Option<f64>,
    metronome: Metronome,
    pub state: State,
    back_notif_buffer: Vec<Notification>,
    connections: ConnectionCollection,
    physics_tick: Box<dyn Fn(&mut State, f64)>,
}

impl Engine {
    pub fn new<InitFn, TickFn>(
        config: &EngineConfig,
        trace_level: TraceLevel,
        new_session_rx: Receiver<Box<dyn SessionBuilder>>,
        init: InitFn,
        physics_tick: TickFn,
    ) -> Self
    where
        InitFn: Fn(&mut State),
        TickFn: Fn(&mut State, f64) + 'static,
    {
        let mut state = State::new();
        let connections = ConnectionCollection::new(new_session_rx, state.root(), 10, trace_level);
        init(&mut state);
        Self {
            should_quit: false,
            quit_after: config.max_game_time,
            metronome: Metronome::default(),
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

        let physics_tick_delta = *self.state.root.physics_tick_delta;
        let physics_ticks_per_network_tick = *self.state.root.physics_ticks_per_network_tick;
        let min_roundtrip_time = *self.state.root.min_roundtrip_time;
        self.metronome.set_params(
            (physics_ticks_per_network_tick as f64) * physics_tick_delta,
            min_roundtrip_time,
        );

        for _ in 0..physics_ticks_per_network_tick {
            (self.physics_tick)(&mut self.state, physics_tick_delta);
        }

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

        if let Some(quit_after) = self.quit_after {
            if *self.state.root.time > quit_after {
                self.should_quit = true;
                info!(
                    "engine has run for {:?}, stopping…",
                    Duration::from_secs_f64(quit_after)
                )
            }
        }

        if !self.should_quit {
            self.metronome.sleep();
        }
        !self.should_quit
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.connections.finalize(&mut self.state);
    }
}
