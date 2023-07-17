use super::*;

/// Configuration for the whole starscape-server program
#[derive(Debug)]
pub struct MasterConfig {
    /// If to abort with success (for example, after showing --help)
    pub happy_exit: bool,
    pub trace_level: TraceLevel,
    pub game: GameConfig,
    pub server: ServerConfig,
}

impl Default for MasterConfig {
    /// NOTE: the true default configuration you get when you run starscape is determined by config_entries(), this is
    /// just an empty struct
    fn default() -> Self {
        Self {
            happy_exit: false,
            trace_level: 1,
            game: GameConfig::default(),
            server: ServerConfig::default(),
        }
    }
}
