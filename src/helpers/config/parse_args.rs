use super::*;

fn transform_arg_name(mut arg_name: &str) -> String {
    for _ in 0..2 {
        if arg_name.starts_with("-") {
            arg_name = &arg_name[1..]
        }
    }
    arg_name.replace("-", "_")
}

fn try_set(
    builder: &mut ConfigBuilder,
    arg_name: &str,
    value_str: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let name = transform_arg_name(arg_name);
    let message = format!("{} command line argument", arg_name);
    if let Some(mut setter) = builder.entry(&name) {
        match (&mut setter, value_str) {
            (ConfigEntrySetter::Bool(ref mut set), Some(value_str)) => {
                match value_str {
                    "true" => return set(true, message),
                    "false" => return set(false, message),
                    _ => (),
                }
            }
            (ConfigEntrySetter::Bool(ref mut set), None) => {
                return set(true, message);
            }
            (ConfigEntrySetter::String(ref mut set), Some(value_str)) => {
                return set(value_str.to_owned(), message);
            }
            (ConfigEntrySetter::Int(ref mut set), Some(value_str)) => {
                if let Ok(i) = value_str.parse::<i64>() {
                    return set(i, message);
                }
            }
            (ConfigEntrySetter::Float(ref mut set), Some(value_str)) => {
                if let Ok(f) = value_str.parse::<f64>() {
                    return set(f, message);
                }
            }
            (_, None) => (),
        }
        match value_str {
            Some(value_str) => Err(format!(
                "{} is not valid for {} (expected: {})",
                value_str, arg_name, setter
            )
            .into()),
            None => Err(format!("{} argument is required for {}", setter, name).into()),
        }
    } else {
        Err(format!("{} is not a valid command line option", arg_name).into())
    }
}

struct Arg {
    pub index: usize,
    pub name: String,
    pub values: Vec<String>,
}

fn parse_list(args: &Vec<String>) -> Result<Vec<Arg>, Box<dyn Error>> {
    let mut parsed = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        if i == 0 {
            if arg.starts_with("-") {
                return Err(format!(
                    "first command line argument {} starts with \"--\", {}",
                    arg, "it should have been the program name"
                )
                .into());
            }
        } else if arg.starts_with("-") {
            parsed.push(Arg {
                index: i,
                name: arg.to_owned(),
                values: Vec::new(),
            });
        } else if let Some(last) = parsed.last_mut() {
            last.values.push(arg.to_owned());
        } else {
            return Err(format!(
                "first command line argument {} is a value not an --option-name",
                arg
            )
            .into());
        }
    }
    Ok(parsed)
}

pub fn parse_args(builder: &mut ConfigBuilder, args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let parsed = parse_list(&args)?;
    for arg in parsed {
        if arg.values.len() > 1 {
            return Err(format!(
                "command line argument {} has multiple values: {}",
                arg.index,
                arg.values.join(" ")
            )
            .into());
        } else if arg.values.len() == 1 {
            try_set(builder, &arg.name, Some(&arg.values[0]))?;
        } else {
            try_set(builder, &arg.name, None)?;
        }
    }
    Ok(())
}
