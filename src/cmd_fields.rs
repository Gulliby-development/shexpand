use crate::error::ShError;
use crate::fields::{self, Item};
use crate::io::{Io, text_arg};
use crate::options::{self, Spec};
use crate::text::{cell, line, table};

pub const USAGE: &str = "shexpand fields [--ifs TEXT] VALUE";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, ShError> {
    let parsed = options::parse(
        args,
        &[Spec {
            name: "--ifs",
            flag: false,
        }],
    )?;

    if parsed.args.len() != 1 {
        return Err(ShError(format!("usage: {USAGE}")));
    }

    let ifs = parsed
        .values
        .get("--ifs")
        .and_then(|values| values.last())
        .map(String::as_str)
        .unwrap_or(" \t\n");
    let value = text_arg(io, &parsed.args[0])?;
    let items: Vec<Item> = value
        .chars()
        .map(|ch| Item::Ch {
            ch,
            quoted: false,
            split: true,
        })
        .collect();
    let fields = fields::split(&items, ifs);

    let rows: Vec<Vec<String>> = fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let text: String = field.iter().map(|(ch, _)| ch).collect();
            vec![(index + 1).to_string(), cell(&text)]
        })
        .collect();
    let mut output = table(&["field", "text"], &rows).join("\n");
    output.push('\n');
    output.push_str(&line("fields", &fields.len().to_string()));
    output.push('\n');

    io.out(&output);
    Ok(0)
}
