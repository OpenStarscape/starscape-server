extern crate config;

use config::{Config, Environment, File};

/// Get the current configuration.
pub fn get() -> Config {
    let mut conf = Config::default();
    conf.set_default("tcp", true).unwrap();
    conf.set_default("websockets", true).unwrap();
    conf.set_default("webrtc", true).unwrap();
    conf.set_default("https", true).unwrap();
    conf.set_default("http_content", "../web/dist").unwrap();
    conf.set_default("max_game_time", 1200.0).unwrap();
    conf.merge(File::with_name("starscape"))
        .unwrap()
        .merge(Environment::with_prefix("STARSCAPE"))
        .unwrap();
    conf
}
