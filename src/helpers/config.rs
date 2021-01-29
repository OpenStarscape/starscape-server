extern crate config;

use config::{File, Environment, Config};

/// Get the current configuration.
pub fn get() -> Config {
    let mut conf = Config::default();
    conf.set_default("max_game_time", 1200.0).unwrap();
    conf.merge(File::with_name("starscape")).unwrap().merge(Environment::with_prefix("STARSCAPE")).unwrap();
    return conf;
}
