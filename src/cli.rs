use crate::cmd_braces;
use crate::cmd_expand;
use crate::cmd_fields;
use crate::cmd_glob;
use crate::cmd_lint;
use crate::cmd_quote;
use crate::cmd_split;
use crate::error::ShError;
use crate::io::Io;
use crate::text::show;

/// Usage lines printed by the singleton `--help` command.
pub const USAGE: &str = "shexpand split LINE\nshexpand braces LINE\nshexpand fields [--ifs TEXT] VALUE\nshexpand glob [--file PATH]... PATTERN...\nshexpand expand [--var NAME=VALUE]... [--ifs TEXT] [--file PATH]... LINE [ARG...]\nshexpand quote [--style single|double] [--always] TEXT...\nshexpand lint LINE\n";

/// Dispatch a command and present adapter errors in the common CLI format.
pub fn run(argv: &[String], io: &dyn Io) -> i32 {
    if argv == ["--help"] {
        io.out(USAGE);
        return 0;
    }

    let Some((command, args)) = argv.split_first() else {
        return report_error(
            io,
            ShError(
                "missing command (split, braces, fields, glob, expand, quote or lint)".to_owned(),
            ),
        );
    };

    let result = match command.as_str() {
        "split" => cmd_split::run(args, io),
        "braces" => cmd_braces::run(args, io),
        "fields" => cmd_fields::run(args, io),
        "glob" => cmd_glob::run(args, io),
        "expand" => cmd_expand::run(args, io),
        "quote" => cmd_quote::run(args, io),
        "lint" => cmd_lint::run(args, io),
        _ => {
            return report_error(io, ShError(format!("unknown command {}", show(command))));
        }
    };

    match result {
        Ok(status) => status,
        Err(error) => report_error(io, error),
    }
}

fn report_error(io: &dyn Io, error: ShError) -> i32 {
    io.err(&format!("shexpand: {}\n", error.0));
    2
}
