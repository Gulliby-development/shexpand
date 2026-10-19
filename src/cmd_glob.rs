use crate::error::ShError;
use crate::io::Io;
use crate::options::{Spec, parse};
use crate::text::{cell, line};
use crate::vfs::Vfs;

pub const USAGE: &str = "shexpand glob [--file PATH]... PATTERN...";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, ShError> {
    let parsed = parse(
        args,
        &[Spec {
            name: "--file",
            flag: false,
        }],
    )?;

    if parsed.args.is_empty() {
        return Err(ShError(format!("usage: {USAGE}")));
    }

    let files = parsed
        .values
        .get("--file")
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let vfs = Vfs::new(files)?;
    let mut output = String::new();

    for operand in &parsed.args {
        output.push_str(&line("pattern", &cell(operand)));
        output.push('\n');

        let cells = pattern_cells(operand);
        let matches = vfs.expand(&cells);
        if matches.is_empty() {
            output.push_str(&line("match", "-"));
            output.push('\n');
        } else {
            for path in matches {
                output.push_str(&line("match", &cell(&path)));
                output.push('\n');
            }
        }
    }

    io.out(&output);
    Ok(0)
}

fn pattern_cells(operand: &str) -> Vec<(char, bool)> {
    let mut chars = operand.chars();
    let mut cells = Vec::new();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(quoted) = chars.next() {
                cells.push((quoted, true));
            } else {
                cells.push(('\\', false));
            }
        } else {
            cells.push((ch, false));
        }
    }

    cells
}
