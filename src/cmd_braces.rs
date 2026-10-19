use crate::braces::expand;
use crate::error::ShError;
use crate::io::{Io, text_arg};
use crate::lexer::lex;
use crate::options::parse;
use crate::render::atom_text;
use crate::text::{cell, line, table};

pub const USAGE: &str = "shexpand braces LINE";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, ShError> {
    let parsed = parse(args, &[])?;
    let [operand] = parsed.args.as_slice() else {
        return Err(ShError(format!("usage: {USAGE}")));
    };

    let input = text_arg(io, operand)?;
    let words = lex(&input)?;
    let mut rows = Vec::new();

    for (index, word) in words.iter().enumerate() {
        for result in expand(&word.atoms)? {
            rows.push(vec![(index + 1).to_string(), cell(&atom_text(&result))]);
        }
    }

    let mut output = table(&["word", "text"], &rows).join("\n");
    output.push('\n');
    output.push_str(&line("words", &rows.len().to_string()));
    output.push('\n');
    io.out(&output);
    Ok(0)
}
