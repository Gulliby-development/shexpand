use crate::error::ShError;
use crate::io::{Io, text_arg};
use crate::lexer::lex;
use crate::options::parse;
use crate::render::parts;
use crate::text::{line, table};

pub const USAGE: &str = "shexpand split LINE";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, ShError> {
    let parsed = parse(args, &[])?;
    let [operand] = parsed.args.as_slice() else {
        return Err(ShError(format!("usage: {USAGE}")));
    };

    let input = text_arg(io, operand)?;
    let words = lex(&input)?;
    let rows = words
        .iter()
        .enumerate()
        .map(|(index, word)| vec![(index + 1).to_string(), parts(&word.atoms).join(" ")])
        .collect::<Vec<_>>();
    let mut output = table(&["word", "parts"], &rows).join("\n");
    output.push('\n');
    output.push_str(&line("words", &words.len().to_string()));
    output.push('\n');
    io.out(&output);
    Ok(0)
}
