use std::collections::BTreeMap;

use crate::error::ShError;
use crate::eval::Scope;
use crate::io::{Io, text_arg};
use crate::lexer::lex;
use crate::options::{self, Spec};
use crate::pipeline::expand_words;
use crate::text::{cell, line, show, table};
use crate::vfs::Vfs;

pub const USAGE: &str =
    "shexpand expand [--var NAME=VALUE]... [--ifs TEXT] [--file PATH]... LINE [ARG...]";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, ShError> {
    let parsed = options::parse(
        args,
        &[
            Spec {
                name: "--var",
                flag: false,
            },
            Spec {
                name: "--ifs",
                flag: false,
            },
            Spec {
                name: "--file",
                flag: false,
            },
        ],
    )?;

    let mut vars = BTreeMap::new();
    for binding in parsed.values.get("--var").map(Vec::as_slice).unwrap_or(&[]) {
        let Some((name, value)) = binding.split_once('=') else {
            return Err(invalid_var(binding));
        };
        if !valid_name(name) {
            return Err(invalid_var(binding));
        }
        vars.insert(name.to_owned(), value.to_owned());
    }

    if parsed.args.is_empty() {
        return Err(ShError(format!("usage: {USAGE}")));
    }

    let ifs = parsed
        .values
        .get("--ifs")
        .and_then(|values| values.last())
        .map(String::as_str)
        .unwrap_or(" \t\n");
    let files = parsed
        .values
        .get("--file")
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let vfs = Vfs::new(files)?;
    let text = text_arg(io, &parsed.args[0])?;
    let words = lex(&text)?;
    let positional = parsed.args[1..].to_vec();
    let mut scope = Scope::new(vars, positional);
    let expanded = expand_words(&words, &mut scope, ifs, &vfs)?;

    let rows: Vec<Vec<String>> = expanded
        .iter()
        .enumerate()
        .map(|(index, word)| vec![(index + 1).to_string(), cell(word)])
        .collect();
    let mut output = table(&["word", "text"], &rows).join("\n");
    output.push('\n');
    for (name, value) in &scope.assigned {
        output.push_str(&line("assigned", &format!("{name}={}", cell(value))));
        output.push('\n');
    }
    output.push_str(&line("words", &expanded.len().to_string()));
    output.push('\n');

    io.out(&output);
    Ok(0)
}

fn invalid_var(binding: &str) -> ShError {
    ShError(format!(
        "invalid --var {} (expected NAME=VALUE with a name of letters, digits and _)",
        show(binding)
    ))
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}
