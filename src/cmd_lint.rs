use crate::error::ShError;
use crate::io::{Io, text_arg};
use crate::lexer::lex;
use crate::linter::lint;
use crate::options::parse;
use crate::text::line;

pub const USAGE: &str = "shexpand lint LINE";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, ShError> {
    let parsed = parse(args, &[])?;
    if parsed.args.len() != 1 {
        return Err(ShError(format!("usage: {USAGE}")));
    }

    let input = text_arg(io, &parsed.args[0])?;
    let words = lex(&input)?;
    let findings = lint(&words);

    let mut output = String::new();
    for finding in &findings {
        output.push_str(&format!(
            "column {}: {} {}\n",
            finding.col, finding.code, finding.message
        ));
    }
    output.push_str(&line("warnings", &findings.len().to_string()));
    output.push('\n');

    io.out(&output);
    if findings.is_empty() { Ok(0) } else { Ok(1) }
}
