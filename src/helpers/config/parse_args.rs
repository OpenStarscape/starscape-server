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
            vec!["--foo", "xyz"],
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
            vec!["--foo", "7"],
        )
    }

    /*
    #[test]
    fn accepts_negative_int() {
        let foo_oneshot = OneshotBeforeDrop::new();
        assert_success(
            vec![
                <dyn ConfigEntry>::new_int("foo", "", 0, move |_, value, source| {
                    assert!(source.is_some());
                    assert_eq!(value, -12);
                    foo_oneshot.fire();
                    Ok(())
                }),
            ],
            vec!["--foo", "-12"],
        )
    }
    */

    #[test]
    fn rejects_invalid_int() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_int("foo", "", 0, move |_, _, _| {
                panic!();
            })],
            vec!["--foo", "abc"],
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
            vec!["--foo", "32.5"],
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
            vec!["--foo", "32"],
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
            vec!["--foo", "abc"],
        )
    }

    #[test]
    fn rejects_float_for_int_entry() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_int("foo", "", 0, move |_, _, _| {
                panic!();
            })],
            vec!["--foo", "32.5"],
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
            vec!["--bar", "7", "--foo", "xyz"],
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
            vec!["--foo", "true", "--bar", "false"],
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
            vec!["--foo", "1"],
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
            vec!["--foo", "xyz"],
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
            vec!["--foo", "bar"],
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
            vec!["--foo-bar", "7"],
        )
    }

    #[test]
    fn rejects_underscore_version_of_name() {
        assert_parse_fail(
            vec![<dyn ConfigEntry>::new_int("foo_bar", "", 0, move |_, _, _| {
                panic!();
            })],
            vec!["--foo_bar", "32.5"],
        )
    }
}
