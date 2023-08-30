use super::*;

pub fn init(state: &mut State, config: &GameConfig) {
    state.root.quit_at.set(config.max_game_time)
}

pub fn tick(state: &mut State) -> bool {
    let physics_dt = *state.root.physics_tick_duration;
    let min_roundtrip_time = *state.root.min_roundtrip_time;
    let time_per_time = *state.root.time_per_time;
    let physics_ticks =
        ((*state.root.network_tick_interval * time_per_time / physics_dt).ceil() as u64).min(5000);
    let effective_target_network_tick = if time_per_time > 0.0 {
        (physics_ticks as f64) * physics_dt / time_per_time
    } else {
        *state.root.network_tick_interval
    };
    state
        .metronome
        .set_params(effective_target_network_tick, min_roundtrip_time);

    for _ in 0..physics_ticks {
        *state.root.time.get_mut() += physics_dt;
        if let Some(pause_at) = *state.root.pause_at {
            if *state.root.time >= pause_at {
                state.root.time_per_time_will_be_set_to(0.0);
                state.root.time_per_time.set(0.0);
                state.root.pause_at.set(None);
                break;
            }
        }
        physics_tick(state, physics_dt);
        if check_pause_conditions(state) {
            state.root.time_per_time_will_be_set_to(0.0);
            state.root.time_per_time.set(0.0);
        }
    }
    if physics_ticks == 0 {
        // Gravity parents should be updated even if the game is paused
        update_gravity_parents(state);
    }

    if let Some(quit_at) = *state.root.quit_at {
        if *state.root.time > quit_at {
            info!(
                "engine has run for {:?}, stopping…",
                Duration::from_secs_f64(quit_at)
            );
            return true;
        }
    }
    false
}
