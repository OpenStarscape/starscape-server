use super::*;

/// These entries will be applied in order of returned vec (NOT in the order the user specifies the entry). All entries
/// Will always be applied.
pub fn config_entries() -> Vec<Box<dyn ConfigEntry>> {
    // We concat! long strings so the vec can be formatted by rustfmt (see https://github.com/rust-lang/rustfmt/issues/3863)
    vec![<dyn ConfigEntry>::new_float(
        "max_game_seconds",
        "seconds to run the game before exiting, or 0 to run until process is killed",
        60.0 * 60.0,
        |conf, time, source| {
            if time > 0.0 {
                conf.max_game_time = Some(time);
                Ok(())
            } else if time == 0.0 {
                conf.max_game_time = None;
                Ok(())
            } else {
                Err(format!("{} should not be negative", source.unwrap()).into())
            }
        },
    )]
}
