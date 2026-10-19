use crate::error::ShError;
use crate::text::show;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub name: &'static str,
    pub flag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub values: BTreeMap<String, Vec<String>>,
    pub args: Vec<String>,
}

pub fn parse(argv: &[String], specs: &[Spec]) -> Result<Parsed, ShError> {
    let mut values = BTreeMap::new();
    for spec in specs {
        values.entry(spec.name.to_owned()).or_insert_with(Vec::new);
    }

    let mut args = Vec::new();
    let mut options = true;
    let mut index = 0;

    while index < argv.len() {
        let arg = &argv[index];

        if options && arg == "--" {
            options = false;
            index += 1;
            continue;
        }

        if !options || !arg.starts_with("--") {
            args.push(arg.clone());
            index += 1;
            continue;
        }

        let (name, inline_value) = match arg.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (arg.as_str(), None),
        };

        let Some(spec) = specs.iter().find(|spec| spec.name == name) else {
            return Err(ShError(format!("unknown option {}", show(arg))));
        };

        let stored_values = values.entry(spec.name.to_owned()).or_insert_with(Vec::new);

        if spec.flag {
            if inline_value.is_some() {
                return Err(ShError(format!("{} takes no value", spec.name)));
            }
            stored_values.push(String::new());
            index += 1;
            continue;
        }

        let value = match inline_value {
            Some(value) => value.to_owned(),
            None => {
                index += 1;
                let Some(value) = argv.get(index) else {
                    return Err(ShError(format!("{} needs a value", spec.name)));
                };
                value.clone()
            }
        };
        stored_values.push(value);
        index += 1;
    }

    Ok(Parsed { values, args })
}
