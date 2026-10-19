use std::cell::{Cell, RefCell};

use shexpand::cmd_braces;
use shexpand::cmd_expand;
use shexpand::cmd_fields;
use shexpand::cmd_glob;
use shexpand::cmd_lint;
use shexpand::cmd_quote;
use shexpand::cmd_split;
use shexpand::error::ShError;
use shexpand::io::Io;

struct RecordingIo {
    input: Vec<u8>,
    reads: Cell<usize>,
    outputs: RefCell<Vec<String>>,
    errors: RefCell<Vec<String>>,
}

impl RecordingIo {
    fn new(input: &[u8]) -> Self {
        Self {
            input: input.to_vec(),
            reads: Cell::new(0),
            outputs: RefCell::new(Vec::new()),
            errors: RefCell::new(Vec::new()),
        }
    }
}

impl Io for RecordingIo {
    fn read_stdin(&self) -> Vec<u8> {
        self.reads.set(self.reads.get() + 1);
        self.input.clone()
    }

    fn out(&self, text: &str) {
        self.outputs.borrow_mut().push(text.to_owned());
    }

    fn err(&self, text: &str) {
        self.errors.borrow_mut().push(text.to_owned());
    }
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn assert_success(io: &RecordingIo, status: i32, expected: &str) {
    assert_eq!(io.reads.get(), 0);
    assert_eq!(io.outputs.borrow().as_slice(), &[expected.to_owned()]);
    assert!(io.errors.borrow().is_empty());
    assert!(status == 0 || status == 1);
}

fn assert_adapter_error(io: &RecordingIo, result: Result<i32, ShError>, message: &str) {
    assert_eq!(result, Err(ShError(message.to_owned())));
    assert_eq!(io.reads.get(), 0);
    assert!(io.outputs.borrow().is_empty());
    assert!(io.errors.borrow().is_empty());
}

#[test]
fn split_command_example_preserves_word_parts_and_comment_handling() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&["cp \"my file\" ${DEST:-/tmp}/x $@ # note"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(
        &io,
        0,
        "word  parts\n1     u[cp]\n2     q[my file]\n3     p[${DEST:-/tmp}] u[/x]\n4     p[$@]\nwords       4\n",
    );
}

#[test]
fn braces_command_example_preserves_ranges_and_original_word_numbers() {
    let io = RecordingIo::new(b"");
    let result = cmd_braces::run(&args(&["a{b,c}d f{08..10}.txt {a..e..2}"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(
        &io,
        0,
        "word  text\n1     [abd]\n1     [acd]\n2     [f08.txt]\n2     [f09.txt]\n2     [f10.txt]\n3     [a]\n3     [c]\n3     [e]\nwords       8\n",
    );
}

#[test]
fn fields_command_example_splits_whitespace_and_delimiters() {
    let io = RecordingIo::new(b"");
    let result = cmd_fields::run(&args(&["--ifs", " :", " a : b::c  "]), &io);

    assert_eq!(result, Ok(0));
    assert_success(
        &io,
        0,
        "field  text\n1      [a]\n2      [b]\n3      []\n4      [c]\nfields      4\n",
    );
}

#[test]
fn glob_command_example_matches_virtual_paths_in_order() {
    let io = RecordingIo::new(b"");
    let result = cmd_glob::run(
        &args(&[
            "--file", "a.txt", "--file", "b.txt", "--file", ".hid", "--file", "d/x.c", "--file",
            "d/y.txt", "*.txt", "d/*", "[ab]*", "z*", ".*", "*/",
        ]),
        &io,
    );

    assert_eq!(result, Ok(0));
    assert_success(
        &io,
        0,
        "pattern     [*.txt]\nmatch       [a.txt]\nmatch       [b.txt]\npattern     [d/*]\nmatch       [d/x.c]\nmatch       [d/y.txt]\npattern     [[ab]*]\nmatch       [a.txt]\nmatch       [b.txt]\npattern     [z*]\nmatch       -\npattern     [.*]\nmatch       [.hid]\npattern     [*/]\nmatch       [d/]\n",
    );
}

#[test]
fn expand_command_example_preserves_pipeline_words_and_unset_defaults() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(
        &args(&[
            "--var",
            "X=a b",
            "--var",
            "E=",
            "--var",
            "HOME=/home/u",
            "echo ~/x $X \"$X\" $E \"$E\" y{1,2} ${U:-q r}",
        ]),
        &io,
    );

    assert_eq!(result, Ok(0));
    assert_success(
        &io,
        0,
        "word  text\n1     [echo]\n2     [/home/u/x]\n3     [a]\n4     [b]\n5     [a b]\n6     []\n7     [y1]\n8     [y2]\n9     [q]\n10    [r]\nwords       10\n",
    );
}

#[test]
fn expand_command_example_preserves_positionals_and_assignment_rows() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["echo \"$@\" ${#} ${N:=7}$N", "p", "q r"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(
        &io,
        0,
        "word  text\n1     [echo]\n2     [p]\n3     [q r]\n4     [2]\n5     [77]\nassigned    N=[7]\nwords       5\n",
    );
}

#[test]
fn expand_command_example_matches_virtual_globs_and_keeps_quoted_pattern() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(
        &args(&[
            "--file",
            "a.txt",
            "--file",
            "b.txt",
            "--file",
            "d/x.c",
            "ls *.txt \"*.txt\" d/?.c",
        ]),
        &io,
    );

    assert_eq!(result, Ok(0));
    assert_success(
        &io,
        0,
        "word  text\n1     [ls]\n2     [a.txt]\n3     [b.txt]\n4     [*.txt]\n5     [d/x.c]\nwords       5\n",
    );
}

#[test]
fn quote_command_example_uses_single_quote_escapes() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&["a", "b c", "it's", ""]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "a 'b c' 'it'\\''s' ''\n");
}

#[test]
fn quote_command_example_uses_double_quote_escapes() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&["--style", "double", "say \"hi\" $HOME", "x"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "\"say \\\"hi\\\" \\$HOME\" x\n");
}

#[test]
fn lint_command_example_sorts_and_formats_findings() {
    let io = RecordingIo::new(b"");
    let result = cmd_lint::run(&args(&["rm -rf $DIR/* \"$*\" ${X:-$Y} $@"]), &io);

    assert_eq!(result, Ok(1));
    assert_success(
        &io,
        1,
        "column 8: W1 unquoted parameter DIR is subject to splitting and pattern matching\ncolumn 16: W3 \"$*\" joins the parameters into one word; \"$@\" keeps them apart\ncolumn 20: W1 unquoted parameter X is subject to splitting and pattern matching\ncolumn 25: W1 unquoted parameter Y is subject to splitting and pattern matching\ncolumn 29: W2 unquoted $@ splits and matches its parameters again; write \"$@\"\nwarnings    5\n",
    );
}

#[test]
fn split_command_accepts_an_empty_line() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&[""]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "word  parts\nwords       0\n");
}

#[test]
fn braces_command_accepts_an_empty_line() {
    let io = RecordingIo::new(b"");
    let result = cmd_braces::run(&args(&[""]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "word  text\nwords       0\n");
}

#[test]
fn fields_command_accepts_an_empty_value() {
    let io = RecordingIo::new(b"");
    let result = cmd_fields::run(&args(&[""]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "field  text\nfields      0\n");
}

#[test]
fn expand_command_accepts_an_empty_line() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&[""]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "word  text\nwords       0\n");
}

#[test]
fn lint_command_excludes_the_length_of_the_parameter_count() {
    let io = RecordingIo::new(b"");
    let result = cmd_lint::run(&args(&["${#}"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "warnings    0\n");
}

#[test]
fn lint_command_reports_an_unquoted_plain_parameter() {
    let io = RecordingIo::new(b"");
    let result = cmd_lint::run(&args(&["$X"]), &io);

    assert_eq!(result, Ok(1));
    assert_success(
        &io,
        1,
        "column 1: W1 unquoted parameter X is subject to splitting and pattern matching\nwarnings    1\n",
    );
}

#[test]
fn glob_command_reports_no_match_for_a_literal_path() {
    let io = RecordingIo::new(b"");
    let result = cmd_glob::run(&args(&["z"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "pattern     [z]\nmatch       -\n");
}

#[test]
fn fields_command_uses_the_last_ifs_option() {
    let io = RecordingIo::new(b"");
    let result = cmd_fields::run(&args(&["--ifs", ",", "--ifs", ":", "a::"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(
        &io,
        0,
        "field  text\n1      [a]\n2      []\nfields      2\n",
    );
}

#[test]
fn quote_command_uses_the_last_style_option() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(
        &args(&["--style", "single", "--style", "double", "--always", "x"]),
        &io,
    );

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "\"x\"\n");
}

#[test]
fn quote_command_accepts_repeated_always_flags() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&["--always", "--always", "x"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "'x'\n");
}

#[test]
fn quote_command_stops_option_parsing_after_terminator() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&["--", "--always"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "--always\n");
}

#[test]
fn quote_command_preserves_unicode_and_embedded_crlf_payload() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&["--always", "é\r\n"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "'é\r\n'\n");
}

#[test]
fn glob_command_matches_a_wildcard_escaped_by_backslash() {
    let io = RecordingIo::new(b"");
    let result = cmd_glob::run(&args(&["--file", "*", "\\*"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "pattern     [\\*]\nmatch       [*]\n");
}

#[test]
fn expand_command_uses_the_last_repeated_variable_value() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["--var", "X=a", "--var", "X=b", "$X"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "word  text\n1     [b]\nwords       1\n");
}

#[test]
fn split_command_reports_exact_arity_usage() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&[]), &io);

    assert_adapter_error(&io, result, "usage: shexpand split LINE");
}

#[test]
fn braces_command_reports_exact_arity_usage() {
    let io = RecordingIo::new(b"");
    let result = cmd_braces::run(&args(&[]), &io);

    assert_adapter_error(&io, result, "usage: shexpand braces LINE");
}

#[test]
fn fields_command_reports_exact_arity_usage() {
    let io = RecordingIo::new(b"");
    let result = cmd_fields::run(&args(&[]), &io);

    assert_adapter_error(&io, result, "usage: shexpand fields [--ifs TEXT] VALUE");
}

#[test]
fn glob_command_reports_exact_arity_usage() {
    let io = RecordingIo::new(b"");
    let result = cmd_glob::run(&args(&[]), &io);

    assert_adapter_error(
        &io,
        result,
        "usage: shexpand glob [--file PATH]... PATTERN...",
    );
}

#[test]
fn expand_command_reports_exact_arity_usage() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&[]), &io);

    assert_adapter_error(
        &io,
        result,
        "usage: shexpand expand [--var NAME=VALUE]... [--ifs TEXT] [--file PATH]... LINE [ARG...]",
    );
}

#[test]
fn quote_command_reports_exact_arity_usage() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&[]), &io);

    assert_adapter_error(
        &io,
        result,
        "usage: shexpand quote [--style single|double] [--always] TEXT...",
    );
}

#[test]
fn lint_command_reports_exact_arity_usage() {
    let io = RecordingIo::new(b"");
    let result = cmd_lint::run(&args(&[]), &io);

    assert_adapter_error(&io, result, "usage: shexpand lint LINE");
}

#[test]
fn quote_command_reports_missing_text_error() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&[]), &io);

    assert_adapter_error(
        &io,
        result,
        "usage: shexpand quote [--style single|double] [--always] TEXT...",
    );
}

#[test]
fn split_command_reports_unknown_option_error() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&["--x", "a"]), &io);

    assert_adapter_error(&io, result, "unknown option \"--x\"");
}

#[test]
fn quote_command_rejects_a_value_for_the_always_flag() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&["--always=1", "a"]), &io);

    assert_adapter_error(&io, result, "--always takes no value");
}

#[test]
fn expand_command_reports_a_missing_variable_option_value() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["--var"]), &io);

    assert_adapter_error(&io, result, "--var needs a value");
}

#[test]
fn expand_command_rejects_a_variable_without_an_equals_sign() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["--var", "X", "echo"]), &io);

    assert_adapter_error(
        &io,
        result,
        "invalid --var \"X\" (expected NAME=VALUE with a name of letters, digits and _)",
    );
}

#[test]
fn quote_command_rejects_an_unknown_style() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&["--style", "round", "a"]), &io);

    assert_adapter_error(
        &io,
        result,
        "invalid --style \"round\" (expected single or double)",
    );
}

#[test]
fn glob_command_rejects_empty_path_components() {
    let io = RecordingIo::new(b"");
    let result = cmd_glob::run(&args(&["--file", "a//b", "a"]), &io);

    assert_adapter_error(
        &io,
        result,
        "invalid --file \"a//b\" (expected a path of names separated by single slashes)",
    );
}

#[test]
fn glob_command_rejects_a_path_that_is_both_file_and_directory() {
    let io = RecordingIo::new(b"");
    let result = cmd_glob::run(&args(&["--file", "a", "--file", "a/b", "a"]), &io);

    assert_adapter_error(&io, result, "\"a/b\" is both a file and a directory");
}

#[test]
fn split_command_reports_unterminated_single_quote() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&["echo 'a"]), &io);

    assert_adapter_error(
        &io,
        result,
        "unterminated single quote starting at column 6",
    );
}

#[test]
fn split_command_reports_unterminated_double_quote() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&[r#"echo "a"#]), &io);

    assert_adapter_error(
        &io,
        result,
        "unterminated double quote starting at column 6",
    );
}

#[test]
fn split_command_reports_unterminated_parameter_brace() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&[r"echo ${A:-b"]), &io);

    assert_adapter_error(&io, result, "unterminated ${ starting at column 6");
}

#[test]
fn split_command_reports_a_trailing_backslash() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&[r"echo a\"]), &io);

    assert_adapter_error(&io, result, "trailing backslash at column 7");
}

#[test]
fn split_command_reports_an_unsupported_operator_and_its_column() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&["a | b"]), &io);

    assert_adapter_error(&io, result, "unsupported operator \"|\" at column 3");
}

#[test]
fn split_command_reports_unescaped_command_substitution() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&[r"echo `ls`"]), &io);

    assert_adapter_error(
        &io,
        result,
        "command substitution is not supported at column 6",
    );
}

#[test]
fn split_command_reports_an_unsupported_special_parameter() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&["echo $?"]), &io);

    assert_adapter_error(&io, result, "unsupported parameter \"$?\" at column 6");
}

#[test]
fn split_command_reports_unsupported_dollar_quoting() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&[r#"echo $"a""#]), &io);

    assert_adapter_error(&io, result, "unsupported quoting after $ at column 6");
}

#[test]
fn split_command_reports_a_bad_parameter_substitution() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&[r"echo ${X^}"]), &io);

    assert_adapter_error(&io, result, "bad substitution \"${X^}\" at column 6");
}

#[test]
fn split_command_rejects_double_quotes_inside_a_double_quoted_operator_word() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&[r#"echo "${X:-"a"}""#]), &io);

    assert_adapter_error(
        &io,
        result,
        "unsupported double quote inside \"${...}\" at column 12",
    );
}

#[test]
fn split_command_reports_parameter_nesting_beyond_eight() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(
        &args(&[r"${A:-${A:-${A:-${A:-${A:-${A:-${A:-${A:-${A:-x}}}}}}}}}"]),
        &io,
    );

    assert_adapter_error(&io, result, "nesting is deeper than 8 at column 41");
}

#[test]
fn braces_command_reports_nesting_beyond_eight() {
    let io = RecordingIo::new(b"");
    let result = cmd_braces::run(&args(&[r"{a,{a,{a,{a,{a,{a,{a,{a,{a,b}}}}}}}}}"]), &io);

    assert_adapter_error(&io, result, "nesting is deeper than 8");
}

#[test]
fn braces_command_reports_more_than_one_thousand_expansions() {
    let io = RecordingIo::new(b"");
    let result = cmd_braces::run(&args(&["{1..2000}"]), &io);

    assert_adapter_error(&io, result, "brace expansion makes more than 1000 words");
}

#[test]
fn expand_command_reports_the_custom_parameter_error_message() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["echo ${Z:?oops}"]), &io);

    assert_adapter_error(&io, result, "Z: oops");
}

#[test]
fn expand_command_reports_default_colon_error_for_empty_parameter() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["echo ${Z:?}"]), &io);

    assert_adapter_error(&io, result, "Z: parameter null or not set");
}

#[test]
fn expand_command_reports_default_error_for_unset_parameter_without_colon() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["echo ${Z?}"]), &io);

    assert_adapter_error(&io, result, "Z: parameter not set");
}

#[test]
fn expand_command_rejects_assignment_to_a_positional_parameter() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["echo ${1:=x}"]), &io);

    assert_adapter_error(&io, result, "1: cannot assign in this way");
}

fn assert_success_after_reads(io: &RecordingIo, status: i32, expected: &str, reads: usize) {
    assert_eq!(io.reads.get(), reads);
    assert_eq!(io.outputs.borrow().as_slice(), &[expected.to_owned()]);
    assert!(io.errors.borrow().is_empty());
    assert!(status == 0 || status == 1);
}

fn assert_adapter_error_after_reads(
    io: &RecordingIo,
    result: Result<i32, ShError>,
    message: &str,
    reads: usize,
) {
    assert_eq!(result, Err(ShError(message.to_owned())));
    assert_eq!(io.reads.get(), reads);
    assert!(io.outputs.borrow().is_empty());
    assert!(io.errors.borrow().is_empty());
}

#[test]
fn split_command_checks_options_before_arity() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&["--x"]), &io);

    assert_adapter_error(&io, result, "unknown option \"--x\"");
}

#[test]
fn quote_command_checks_style_before_arity() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&["--style", "round"]), &io);

    assert_adapter_error(
        &io,
        result,
        "invalid --style \"round\" (expected single or double)",
    );
}

#[test]
fn expand_command_validates_variables_before_files_and_stdin() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["--file", "a//b", "--var", "X", "-"]), &io);

    assert_adapter_error(
        &io,
        result,
        "invalid --var \"X\" (expected NAME=VALUE with a name of letters, digits and _)",
    );
}

#[test]
fn expand_command_checks_arity_before_constructing_virtual_files() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["--file", "a//b"]), &io);

    assert_adapter_error(
        &io,
        result,
        "usage: shexpand expand [--var NAME=VALUE]... [--ifs TEXT] [--file PATH]... LINE [ARG...]",
    );
}

#[test]
fn expand_command_validates_files_before_reading_stdin() {
    let io = RecordingIo::new(b"ignored");
    let result = cmd_expand::run(&args(&["--file", "a//b", "-"]), &io);

    assert_adapter_error(
        &io,
        result,
        "invalid --file \"a//b\" (expected a path of names separated by single slashes)",
    );
}

#[test]
fn glob_command_checks_arity_before_constructing_virtual_files() {
    let io = RecordingIo::new(b"");
    let result = cmd_glob::run(&args(&["--file", "a//b"]), &io);

    assert_adapter_error(
        &io,
        result,
        "usage: shexpand glob [--file PATH]... PATTERN...",
    );
}

#[test]
fn quote_command_reports_the_first_invalid_repeated_style() {
    let io = RecordingIo::new(b"");
    let result = cmd_quote::run(&args(&["--style", "round", "--style", "single", "a"]), &io);

    assert_adapter_error(
        &io,
        result,
        "invalid --style \"round\" (expected single or double)",
    );
}

#[test]
fn split_command_checks_arity_before_reading_stdin() {
    let io = RecordingIo::new(b"ignored");
    let result = cmd_split::run(&args(&["-", "extra"]), &io);

    assert_adapter_error(&io, result, "usage: shexpand split LINE");
}

#[test]
fn split_command_reads_bom_prefixed_crlf_stdin_once() {
    let io = RecordingIo::new(&[0xef, 0xbb, 0xbf, b'a', b'\r', b'\n']);
    let result = cmd_split::run(&args(&["-"]), &io);

    assert_eq!(result, Ok(0));
    assert_success_after_reads(&io, 0, "word  parts\n1     u[a]\nwords       1\n", 1);
}

#[test]
fn split_command_rejects_malformed_utf8_stdin_after_one_read() {
    let io = RecordingIo::new(&[0x65, 0x63, 0xff, 0x0a]);
    let result = cmd_split::run(&args(&["-"]), &io);

    assert_adapter_error_after_reads(&io, result, "<stdin>: input is not valid UTF-8", 1);
}

#[test]
fn split_command_does_not_read_stdin_for_a_literal_operand() {
    let io = RecordingIo::new(&[0xff]);
    let result = cmd_split::run(&args(&["a"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "word  parts\n1     u[a]\nwords       1\n");
}

#[test]
fn quote_command_treats_dash_as_literal_text_without_reading_stdin() {
    let io = RecordingIo::new(b"not used");
    let result = cmd_quote::run(&args(&["-"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "-\n");
}

#[test]
fn glob_command_treats_dash_as_a_literal_pattern_without_reading_stdin() {
    let io = RecordingIo::new(b"not used");
    let result = cmd_glob::run(&args(&["-"]), &io);

    assert_eq!(result, Ok(0));
    assert_success(&io, 0, "pattern     [-]\nmatch       -\n");
}

#[test]
fn braces_command_reads_accepted_stdin_once() {
    let io = RecordingIo::new(b"a{b,c}\n");
    let result = cmd_braces::run(&args(&["-"]), &io);

    assert_eq!(result, Ok(0));
    assert_success_after_reads(
        &io,
        0,
        "word  text\n1     [ab]\n1     [ac]\nwords       2\n",
        1,
    );
}

#[test]
fn fields_command_reads_accepted_stdin_once() {
    let io = RecordingIo::new(b"a::\n");
    let result = cmd_fields::run(&args(&["--ifs", ":", "-"]), &io);

    assert_eq!(result, Ok(0));
    assert_success_after_reads(
        &io,
        0,
        "field  text\n1      [a]\n2      []\nfields      2\n",
        1,
    );
}

#[test]
fn expand_command_reads_accepted_stdin_once() {
    let io = RecordingIo::new(b"echo ok\n");
    let result = cmd_expand::run(&args(&["-"]), &io);

    assert_eq!(result, Ok(0));
    assert_success_after_reads(
        &io,
        0,
        "word  text\n1     [echo]\n2     [ok]\nwords       2\n",
        1,
    );
}

#[test]
fn lint_command_reads_accepted_stdin_once() {
    let io = RecordingIo::new(b"$X\n");
    let result = cmd_lint::run(&args(&["-"]), &io);

    assert_eq!(result, Ok(1));
    assert_success_after_reads(
        &io,
        1,
        "column 1: W1 unquoted parameter X is subject to splitting and pattern matching\nwarnings    1\n",
        1,
    );
}

#[test]
fn split_command_does_not_write_partial_output_after_a_later_lex_error() {
    let io = RecordingIo::new(b"");
    let result = cmd_split::run(&args(&["ok '"]), &io);

    assert_adapter_error(
        &io,
        result,
        "unterminated single quote starting at column 4",
    );
}

#[test]
fn braces_command_does_not_write_partial_output_after_later_expansion_error() {
    let io = RecordingIo::new(b"");
    let result = cmd_braces::run(&args(&["ok {1..2000}"]), &io);

    assert_adapter_error(&io, result, "brace expansion makes more than 1000 words");
}

#[test]
fn expand_command_does_not_write_assigned_rows_after_later_parameter_error() {
    let io = RecordingIo::new(b"");
    let result = cmd_expand::run(&args(&["ok ${N:=7} ${Z:?oops}"]), &io);

    assert_adapter_error(&io, result, "Z: oops");
}
