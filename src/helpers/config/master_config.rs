use super::*;

/// Configuration for the whole starscape-server program
pub struct MasterConfig {
    pub max_game_time: f64,
    pub server: ServerConfig,
}

impl Default for MasterConfig {
    /// NOTE: the true default configuration you get when you run starscape is determined by config_entries(), this is
    /// just an empty struct
    fn default() -> Self {
        Self {
            max_game_time: 0.0,
            server: ServerConfig {
                tcp: None,
                http: None,
            },
        }
    }
}
