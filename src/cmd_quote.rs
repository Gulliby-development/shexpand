use crate::error::ShError;
use crate::io::Io;
use crate::options::{Spec, parse};
use crate::quoter::quote;
use crate::text::show;

pub const USAGE: &str = "shexpand quote [--style single|double] [--always] TEXT...";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, ShError> {
    let parsed = parse(
        args,
        &[
            Spec {
                name: "--style",
                flag: false,
            },
            Spec {
                name: "--always",
                flag: true,
            },
        ],
    )?;

    let mut double = false;
    if let Some(styles) = parsed.values.get("--style") {
        for style in styles {
            double = match style.as_str() {
                "single" => false,
                "double" => true,
                _ => {
                    return Err(ShError(format!(
                        "invalid --style {} (expected single or double)",
                        show(style)
                    )));
                }
            };
        }
    }

    if parsed.args.is_empty() {
        return Err(ShError(format!("usage: {USAGE}")));
    }

    let always = parsed
        .values
        .get("--always")
        .is_some_and(|values| !values.is_empty());
    let mut output = String::new();
    for (index, text) in parsed.args.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        output.push_str(&quote(text, double, always));
    }
    output.push('\n');
    io.out(&output);

    Ok(0)
}
