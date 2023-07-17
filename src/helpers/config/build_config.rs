use super::*;

/// logging level to display (each level also infludes messages from lower levels)
/// see config option help for specifics
pub type TraceLevel = i32;

/// Applied in order of returned vec (NOT in the order the user specifies the entry). All entries are always be applied.
fn master_config_entries() -> Vec<Box<dyn ConfigEntry>> {
    // We concat! long strings so the vec can be formatted by rustfmt (see https://github.com/rust-lang/rustfmt/issues/3863)
    vec![<dyn ConfigEntry>::new_int(
        "trace",
        concat!(
            "logging level to display (each level includes messages from lower levels)\n",
            "0: none\n",
            "1: configuration, startup, shutdown, and other basic info\n",
            "2: when connections are opened or closed\n",
            "3: all messages to and from clients except property updates\n",
            "4: all messages including updates",
        ),
        2,
        |conf, value, source| {
            if value >= 0 && value <= 4 {
                conf.trace_level = value as TraceLevel;
                Ok(())
            } else {
                Err(format!("{} should be between 0 and 4", source.unwrap()).into())
            }
        },
    )]
}

pub fn build_config_with(
    args: Vec<String>,
    fs: Filesystem,
) -> Result<MasterConfig, Box<dyn Error>> {
    // TODO: use environment variables
    // TODO: accumulate multiple config errors
    // TODO: IPv6 config???
    let mut entries = Vec::new();
    entries.append(&mut master_config_entries());
    entries.append(&mut game_config_entries());
    entries.append(&mut server_config_entries());
    let mut builder = ConfigBuilder::new(entries);
    if fs.is_file(DEFAULT_TOML_PATH) {
        load_toml(DEFAULT_TOML_PATH, &mut builder, fs)?;
    }
    parse_args(&mut builder, args)?;
    let mut conf = MasterConfig::default();
    builder.apply_to(&mut conf)?;
    Ok(conf)
}

/// Build a configuration
pub fn build_config() -> Result<MasterConfig, Box<dyn Error>> {
    build_config_with(std::env::args().collect(), real_filesystem())
}
