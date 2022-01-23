use super::*;

fn arg_name_to_entry_name(mut arg_name: &str) -> String {
    for _ in 0..2 {
        if arg_name.starts_with("-") {
            arg_name = &arg_name[1..]
        }
    }
    arg_name.replace("-", "_")
}

fn entry_name_to_arg_name(entry_name: &str) -> String {
    "--".to_owned() + &entry_name.replace("_", "-")
}

fn try_set(
    builder: &mut ConfigBuilder,
    arg_name: &str,
    value_str: &str,
) -> Result<(), Box<dyn Error>> {
    let name = arg_name_to_entry_name(arg_name);
    let message = format!("{} command line argument", arg_name);
    if let Some(mut setter) = builder.entry(&name) {
        match &mut setter {
            ConfigEntrySetter::Bool(ref mut set) => match value_str {
                "true" => set(true, message),
                "false" => set(false, message),
                _ => Err(format!(
                    "{} value {} is not a valid bool (true or false)",
                    arg_name, value_str
                )
                .into()),
            },
            ConfigEntrySetter::String(ref mut set) => set(value_str.to_owned(), message),
            ConfigEntrySetter::Int(ref mut set) => {
                if let Ok(i) = value_str.parse::<i64>() {
                    set(i, message)
                } else {
                    Err(format!("{} value {} is not a valid int", arg_name, value_str).into())
                }
            }
            ConfigEntrySetter::Float(ref mut set) => {
                if let Ok(f) = value_str.parse::<f64>() {
                    set(f, message)
                } else {
                    Err(format!("{} value {} is not a valid float", arg_name, value_str).into())
                }
            }
        }
    } else {
        Err(format!("{} is not a valid command line option", arg_name).into())
    }
}

struct Arg {
    pub name: String,
    pub value: String,
}

struct Parsed {
    pub help: bool,
    pub parsed: Vec<Arg>,
}

fn parse_list(args: &Vec<String>) -> Result<Parsed, Box<dyn Error>> {
    let mut parsed = Vec::new();
    let mut help = false;
    for (i, arg) in args.iter().enumerate() {
        if i == 0 {
            if arg.starts_with("-") {
                return Err(format!(
                    "first command line argument {} starts with \"--\", {}",
                    arg, "it should have been the program name"
                )
                .into());
            }
        } else if arg == "-h" || arg == "--help" {
            help = true;
        } else if let (true, Some(split)) = (arg.starts_with("-"), arg.find('=')) {
            parsed.push(Arg {
                name: arg[..split].to_owned(),
                // + 1 should be safe because we always split on =, which is a 1 byte ASCII character
                value: arg[split + 1..].to_owned(),
            })
        } else {
            return Err(format!(
                "invalid command line argument {}, should look like --config-option=value",
                i
            )
            .into());
        }
    }
    Ok(Parsed { help, parsed })
}

pub fn parse_args(builder: &mut ConfigBuilder, args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let parsed = parse_list(&args)?;
    if parsed.help {
        let prog_name = args
            .first()
            .map(String::as_str)
            .unwrap_or("starscape-server");
        println!("OpenStarscape server");
        println!();
        println!("USAGE: {} --help", prog_name);
        println!("       {} --config-option=value ...", prog_name);
        println!();
        println!("CONFIG OPTIONS:");
        for entry in builder.help_data() {
            let name = entry_name_to_arg_name(&entry.name);
            let mut help = entry.help_text.clone();
            if help.contains("\n") {
                help += "\n";
            } else {
                help += " ";
            }
            help += &format!("(default: {})", entry.default_value);
            help = help
                .lines()
                .enumerate()
                .map(|(i, line)| match i {
                    0 => line.to_owned(),
                    _ => format!("\n{:<31}{}", " ", line),
                })
                .collect();
            println!("  {:<26} {} ", format!("{}=_", name), help);
            println!();
        }
        println!("CONFIG FILE:");
        println!(
            "  Config options may also be specified in a config file named {}",
            DEFAULT_TOML_PATH
        );
        println!("  Options set in this file are in the format config_option=\"value\"");
        println!("  Strings (but not numbers and bools) must be quoted, -- is not used and _ is used instead of -");
        builder.set_happy_exit();
    } else {
        for arg in &parsed.parsed {
            try_set(builder, &arg.name, &arg.value)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_success(entries: Vec<Box<dyn ConfigEntry>>, args: Vec<&str>) {
        let mut builder = ConfigBuilder::new(entries);
        let args = std::iter::once("program_name")
            .chain(args.into_iter())
            .map(ToOwned::to_owned)
            .collect();
        parse_args(&mut builder, args).unwrap();
        let mut conf = MasterConfig::default();
        builder.apply_to(&mut conf).unwrap();
    }

    fn assert_parse_fail(entries: Vec<Box<dyn ConfigEntry>>, args: Vec<&str>) {
        let mut builder = ConfigBuilder::new(entries);
        let args = std::iter::once("program_name")
            .chain(args.into_iter())
            .map(ToOwned::to_owned)
            .collect();
        parse_args(&mut builder, args).unwrap_err();
    }

    fn assert_apply_fail(entries: Vec<Box<dyn ConfigEntry>>, args: Vec<&str>) {
        let mut builder = ConfigBuilder::new(entries);
        let args = std::iter::once("program_name")
            .chain(args.into_iter())
            .map(ToOwned::to_owned)
            .collect();
        parse_args(&mut builder, args).unwrap();
        let mut conf = MasterConfig::default();
        builder.apply_to(&mut conf).unwrap_err();
    }

    #[test]
    fn accepts_no_args() {
        assert_success(vec![], vec![])
    }

    #[test]
    fn accepts_string() {
        let foo_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![<dyn ConfigEntry>::new_string(
                "foo",
                "",
                "abc",
                move |_, value, source| {
                    assert!(source.is_some());
                    assert_eq!(value, "xyz");
                    foo_oneshot.fire();
                    Ok(())
                },
            )],
            vec!["--foo=xyz"],
        )
    }

    #[test]
    fn rejects_string_without_argument() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_string(
                "foo",
                "",
                "abc",
                move |_, _, _| {
                    panic!();
                },
            )],
            vec!["--foo"],
        )
    }

    #[test]
    fn accepts_string_with_empty_argument() {
        let foo_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![<dyn ConfigEntry>::new_string(
                "foo",
                "",
                "abc",
                move |_, value, source| {
                    assert!(source.is_some());
                    assert_eq!(value, "");
                    foo_oneshot.fire();
                    Ok(())
                },
            )],
            vec!["--foo="],
        )
    }

    #[test]
    fn accepts_int() {
        let foo_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![<dyn ConfigEntry>::new_int(
                "foo",
                "",
                0,
                move |_, value, source| {
                    assert!(source.is_some());
                    assert_eq!(value, 7);
                    foo_oneshot.fire();
                    Ok(())
                },
            )],
            vec!["--foo=7"],
        )
    }

    #[test]
    fn accepts_negative_int() {
        let foo_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![<dyn ConfigEntry>::new_int(
                "foo",
                "",
                0,
                move |_, value, source| {
                    assert!(source.is_some());
                    assert_eq!(value, -12);
                    foo_oneshot.fire();
                    Ok(())
                },
            )],
            vec!["--foo=-12"],
        )
    }

    #[test]
    fn rejects_invalid_int() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_int("foo", "", 0, move |_, _, _| {
                panic!();
            })],
            vec!["--foo=abc"],
        )
    }

    #[test]
    fn accepts_float() {
        let foo_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![<dyn ConfigEntry>::new_float(
                "foo",
                "",
                0.0,
                move |_, value, source| {
                    assert!(source.is_some());
                    assert_eq!(value, 32.5);
                    foo_oneshot.fire();
                    Ok(())
                },
            )],
            vec!["--foo=32.5"],
        )
    }

    #[test]
    fn accepts_int_for_float_entry() {
        let foo_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![<dyn ConfigEntry>::new_float(
                "foo",
                "",
                0.0,
                move |_, value, source| {
                    assert!(source.is_some());
                    assert_eq!(value, 32.0);
                    foo_oneshot.fire();
                    Ok(())
                },
            )],
            vec!["--foo=32"],
        )
    }

    #[test]
    fn rejects_invalid_float() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_float(
                "foo",
                "",
                0.0,
                move |_, _, _| {
                    panic!();
                },
            )],
            vec!["--foo=abc"],
        )
    }

    #[test]
    fn rejects_float_for_int_entry() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_int("foo", "", 0, move |_, _, _| {
                panic!();
            })],
            vec!["--foo=32.5"],
        )
    }

    #[test]
    fn accepts_multiple() {
        let foo_oneshot = OneshotBeforeDrop::new();
        let bar_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![
                <dyn ConfigEntry>::new_string("foo", "", "abc", move |_, value, source| {
                    assert!(source.is_some());
                    assert_eq!(value, "xyz");
                    foo_oneshot.fire();
                    Ok(())
                }),
                <dyn ConfigEntry>::new_int("bar", "", 0, move |_, value, source| {
                    assert!(source.is_some());
                    assert_eq!(value, 7);
                    bar_oneshot.fire();
                    Ok(())
                }),
            ],
            vec!["--bar=7", "--foo=xyz"],
        )
    }

    #[test]
    fn accepts_bools() {
        let foo_oneshot = OneshotBeforeDrop::new();
        let bar_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![
                <dyn ConfigEntry>::new_bool("foo", "", false, move |_, value, source| {
                    assert!(source.is_some());
                    assert!(value);
                    foo_oneshot.fire();
                    Ok(())
                }),
                <dyn ConfigEntry>::new_bool("bar", "", true, move |_, value, source| {
                    assert!(source.is_some());
                    assert!(!value);
                    bar_oneshot.fire();
                    Ok(())
                }),
            ],
            vec!["--foo=true", "--bar=false"],
        )
    }

    #[test]
    fn rejects_bool_without_argument() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_bool(
                "foo",
                "",
                false,
                move |_, _, _| {
                    panic!();
                },
            )],
            vec!["--foo"],
        )
    }

    #[test]
    fn rejects_bool_with_empty_argument() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_bool(
                "foo",
                "",
                false,
                move |_, _, _| {
                    panic!();
                },
            )],
            vec!["--foo="],
        )
    }

    #[test]
    fn rejects_invalid_bool_value() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_bool(
                "foo",
                "",
                false,
                move |_, _, _| {
                    panic!();
                },
            )],
            vec!["--foo=1"],
        )
    }

    #[test]
    fn accepts_enum() {
        let foo_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![<dyn ConfigEntry>::new_enum(
                "foo",
                "",
                vec![
                    <dyn ConfigEntry>::new_enum_variant("abc", "", |_, _| {
                        panic!();
                    }),
                    <dyn ConfigEntry>::new_enum_variant("xyz", "", move |_, source| {
                        assert!(source.is_some());
                        foo_oneshot.fire();
                    }),
                    <dyn ConfigEntry>::new_enum_variant("ijk", "", |_, _| {
                        panic!();
                    }),
                ],
            )],
            vec!["--foo=xyz"],
        )
    }

    #[test]
    fn rejects_invalid_enum() {
        assert_apply_fail(
            vec![<dyn ConfigEntry>::new_enum(
                "foo",
                "",
                vec![
                    <dyn ConfigEntry>::new_enum_variant("abc", "", |_, _| {
                        panic!();
                    }),
                    <dyn ConfigEntry>::new_enum_variant("xyz", "", |_, _| {
                        panic!();
                    }),
                    <dyn ConfigEntry>::new_enum_variant("ijk", "", |_, _| {
                        panic!();
                    }),
                ],
            )],
            vec!["--foo=bar"],
        )
    }

    #[test]
    fn accepts_multi_word_name_with_dash() {
        let foo_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![<dyn ConfigEntry>::new_int(
                "foo_bar",
                "",
                0,
                move |_, value, source| {
                    assert!(source.is_some());
                    assert_eq!(value, 7);
                    foo_oneshot.fire();
                    Ok(())
                },
            )],
            vec!["--foo-bar=7"],
        )
    }

    #[test]
    fn rejects_underscore_version_of_name() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_int(
                "foo_bar",
                "",
                0,
                move |_, _, _| {
                    panic!();
                },
            )],
            vec!["--foo_bar=32.5"],
        )
    }
}
