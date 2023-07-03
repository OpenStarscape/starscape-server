use super::*;

/// Parameters to create an engine
#[derive(Debug)]
pub struct EngineConfig {
    // The number of in-game seconds before the engine exits
    pub max_game_time: Option<f64>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_game_time: None,
        }
    }
}

/// Applied in order of returned vec (NOT in the order the user specifies the entry). All entries are always be applied.
pub fn engine_config_entries() -> Vec<Box<dyn ConfigEntry>> {
    // We concat! long strings so the vec can be formatted by rustfmt (see https://github.com/rust-lang/rustfmt/issues/3863)
    vec![<dyn ConfigEntry>::new_float(
        "max_game_seconds",
        "seconds to run the game before exiting, or 0 to run until process is killed",
        60.0 * 60.0,
        |conf, time, source| {
            if time > 0.0 {
                conf.engine.max_game_time = Some(time);
                Ok(())
            } else if time == 0.0 {
                conf.engine.max_game_time = None;
                Ok(())
            } else {
                Err(format!("{} should not be negative", source.unwrap()).into())
            }
        },
    )]
}
