use super::*;

#[test]
fn default_max_game_seconds() {
    let fs = MockFilesystem::new();
    let conf = build_config_with(fs.boxed()).unwrap();
    assert!(conf.max_game_time.unwrap() > 0.0);
}

#[test]
fn can_config_max_game_seconds() {
    let fs = MockFilesystem::new().add_file("starscape.toml", "max_game_seconds = 44.0");
    let conf = build_config_with(fs.boxed()).unwrap();
    assert_eq!(conf.max_game_time.unwrap(), 44.0);
}

#[test]
fn can_config_max_game_seconds_with_int() {
    let fs = MockFilesystem::new().add_file("starscape.toml", "max_game_seconds = 44");
    let conf = build_config_with(fs.boxed()).unwrap();
    assert_eq!(conf.max_game_time.unwrap(), 44.0);
}

#[test]
fn zero_max_game_seconds_is_none() {
    let fs = MockFilesystem::new().add_file("starscape.toml", "max_game_seconds = 0");
    let conf = build_config_with(fs.boxed()).unwrap();
    assert!(conf.max_game_time.is_none());
}

#[test]
fn can_not_config_max_game_seconds_with_string() {
    let fs = MockFilesystem::new().add_file("starscape.toml", "max_game_seconds = \"44\"");
    build_config_with(fs.boxed()).unwrap_err();
}
