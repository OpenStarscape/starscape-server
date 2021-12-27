extern crate toml;

use super::*;

pub const DEFAULT_TOML_PATH: &str = "starscape.toml";

pub fn try_set(
    builder: &mut ConfigBuilder,
    name: &str,
    value: toml::Value,
) -> Result<(), Box<dyn Error>> {
    if let Some(mut setter) = builder.entry(name) {
        match &mut setter {
            ConfigEntrySetter::Bool(ref mut s) => {
                if let toml::Value::Boolean(v) = value {
                    return s(v);
                }
            }
            ConfigEntrySetter::String(ref mut s) => {
                if let toml::Value::String(v) = value {
                    return s(v);
                }
            }
            ConfigEntrySetter::Int(ref mut s) => {
                if let toml::Value::Integer(v) = value {
                    return s(v);
                }
            }
            ConfigEntrySetter::Float(ref mut s) => match value {
                toml::Value::Float(v) => return s(v),
                toml::Value::Integer(v) => return s(v as f64),
                _ => (),
            },
        }
        Err(format!("{} is not valid for {} (expected: {})", value, name, setter).into())
    } else {
        Err(format!("{} is not a valid option", name).into())
    }
}

pub fn load_toml(
    path: &str,
    builder: &mut ConfigBuilder,
    fs: Filesystem,
) -> Result<(), Box<dyn Error>> {
    let contents = fs.read_to_string(path)?;
    let parsed = contents.parse::<toml::Value>()?;
    match parsed {
        toml::Value::Table(table) => {
            for (name, value) in table {
                try_set(builder, &name, value).map_err(|e| format!("{}: {}", path, e))?;
                // TODO: accumulate errors instead of returning on the first one
            }
            Ok(())
        }
        _ => Err(format!("toplevel value of {} is not a table", path).into()),
    }
}
