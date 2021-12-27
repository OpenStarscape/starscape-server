use super::*;

#[test]
fn default_max_game_time() {
    let fs = MockFilesystem::new();
    let conf = build_config_with(fs.boxed()).unwrap();
    assert!(conf.max_game_time > 0.0);
}

#[test]
fn can_config_max_game_time() {
    let fs = MockFilesystem::new().add_file("starscape.toml", "max_game_time = 44.0");
    let conf = build_config_with(fs.boxed()).unwrap();
    assert_eq!(conf.max_game_time, 44.0);
}

#[test]
fn can_config_max_game_time_with_int() {
    let fs = MockFilesystem::new().add_file("starscape.toml", "max_game_time = 44");
    let conf = build_config_with(fs.boxed()).unwrap();
    assert_eq!(conf.max_game_time, 44.0);
}

#[test]
fn default_cert_and_key_paths() {
    let fs = MockFilesystem::new().add_file("starscape.toml", "https = true");
    let conf = build_config_with(fs.boxed()).unwrap();
    match conf.server.http.unwrap().server_type {
        HttpServerType::Encrypted(https) => {
            assert!(https.cert_path.contains("cert"));
            assert!(https.key_path.contains("key"));
        }
        _ => panic!("config has unencrypted server"),
    };
}
