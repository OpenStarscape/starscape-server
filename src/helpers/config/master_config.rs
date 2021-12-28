use super::*;

/// Configuration for the whole starscape-server program
#[derive(Debug)]
pub struct MasterConfig {
    pub max_game_time: Option<f64>,
    pub server: ServerConfig,
}

impl Default for MasterConfig {
    /// NOTE: the true default configuration you get when you run starscape is determined by config_entries(), this is
    /// just an empty struct
    fn default() -> Self {
        Self {
            max_game_time: None,
            server: ServerConfig::default(),
        }
    }
}
