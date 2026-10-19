use shexpand::io::Io;
use std::cell::{Cell, RefCell};
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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

fn owned_args(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|arg| (*arg).to_owned()).collect()
}

fn assert_success(
    argv: &[&str],
    input: &[u8],
    status: i32,
    expected_stdout: &str,
    expected_reads: usize,
) {
    let io = RecordingIo::new(input);

    assert_eq!(shexpand::cli::run(&owned_args(argv), &io), status);
    assert_eq!(io.reads.get(), expected_reads);
    assert_eq!(
        io.outputs.borrow().as_slice(),
        &[expected_stdout.to_owned()]
    );
    assert!(io.errors.borrow().is_empty());
}

fn assert_error(argv: &[&str], input: &[u8], expected_stderr: &str) {
    let io = RecordingIo::new(input);

    assert_eq!(shexpand::cli::run(&owned_args(argv), &io), 2);
    assert_eq!(io.reads.get(), 0);
    assert!(io.outputs.borrow().is_empty());
    assert_eq!(io.errors.borrow().as_slice(), &[expected_stderr.to_owned()]);
}

#[test]
fn cli_help_prints_the_exact_usage_once() {
    assert_success(
        &["--help"],
        b"",
        0,
        "shexpand split LINE\nshexpand braces LINE\nshexpand fields [--ifs TEXT] VALUE\nshexpand glob [--file PATH]... PATTERN...\nshexpand expand [--var NAME=VALUE]... [--ifs TEXT] [--file PATH]... LINE [ARG...]\nshexpand quote [--style single|double] [--always] TEXT...\nshexpand lint LINE\n",
        0,
    );
}

#[test]
fn cli_missing_command_is_a_single_prefixed_error() {
    assert_error(
        &[],
        b"",
        "shexpand: missing command (split, braces, fields, glob, expand, quote or lint)\n",
    );
}

#[test]
fn cli_unknown_command_is_sanitized_and_prefixed() {
    assert_error(&["frob"], b"", "shexpand: unknown command \"frob\"\n");
}

#[test]
fn cli_only_accepts_help_as_the_single_argument() {
    assert_error(
        &["--help", "split"],
        b"",
        "shexpand: unknown command \"--help\"\n",
    );
}

#[test]
fn cli_sanitizes_non_ascii_and_line_ends_in_unknown_commands() {
    assert_error(&["é\n"], b"", "shexpand: unknown command \"??\"\n");
}

#[test]
fn cli_split_example_preserves_parts_and_word_layout() {
    assert_success(
        &["split", "cp \"my file\" ${DEST:-/tmp}/x $@ # note"],
        b"",
        0,
        "word  parts\n1     u[cp]\n2     q[my file]\n3     p[${DEST:-/tmp}] u[/x]\n4     p[$@]\nwords       4\n",
        0,
    );
}

#[test]
fn cli_braces_example_preserves_expansion_order_and_padding() {
    assert_success(
        &["braces", "a{b,c}d f{08..10}.txt {a..e..2}"],
        b"",
        0,
        "word  text\n1     [abd]\n1     [acd]\n2     [f08.txt]\n2     [f09.txt]\n2     [f10.txt]\n3     [a]\n3     [c]\n3     [e]\nwords       8\n",
        0,
    );
}

#[test]
fn cli_fields_example_preserves_empty_delimited_fields() {
    assert_success(
        &["fields", "--ifs", " :", " a : b::c  "],
        b"",
        0,
        "field  text\n1      [a]\n2      [b]\n3      []\n4      [c]\nfields      4\n",
        0,
    );
}

#[test]
fn cli_glob_example_preserves_matches_and_pattern_order() {
    assert_success(
        &[
            "glob", "--file", "a.txt", "--file", "b.txt", "--file", ".hid", "--file", "d/x.c",
            "--file", "d/y.txt", "*.txt", "d/*", "[ab]*", "z*", ".*", "*/",
        ],
        b"",
        0,
        "pattern     [*.txt]\nmatch       [a.txt]\nmatch       [b.txt]\npattern     [d/*]\nmatch       [d/x.c]\nmatch       [d/y.txt]\npattern     [[ab]*]\nmatch       [a.txt]\nmatch       [b.txt]\npattern     [z*]\nmatch       -\npattern     [.*]\nmatch       [.hid]\npattern     [*/]\nmatch       [d/]\n",
        0,
    );
}

#[test]
fn cli_expand_example_expands_variables_fields_and_braces() {
    assert_success(
        &[
            "expand",
            "--var",
            "X=a b",
            "--var",
            "E=",
            "--var",
            "HOME=/home/u",
            "echo ~/x $X \"$X\" $E \"$E\" y{1,2} ${U:-q r}",
        ],
        b"",
        0,
        "word  text\n1     [echo]\n2     [/home/u/x]\n3     [a]\n4     [b]\n5     [a b]\n6     []\n7     [y1]\n8     [y2]\n9     [q]\n10    [r]\nwords       10\n",
        0,
    );
}

#[test]
fn cli_expand_example_keeps_quoted_positionals_and_assignments() {
    assert_success(
        &["expand", "echo \"$@\" ${#} ${N:=7}$N", "p", "q r"],
        b"",
        0,
        "word  text\n1     [echo]\n2     [p]\n3     [q r]\n4     [2]\n5     [77]\nassigned    N=[7]\nwords       5\n",
        0,
    );
}

#[test]
fn cli_expand_example_matches_virtual_files_and_respects_quotes() {
    assert_success(
        &[
            "expand",
            "--file",
            "a.txt",
            "--file",
            "b.txt",
            "--file",
            "d/x.c",
            "ls *.txt \"*.txt\" d/?.c",
        ],
        b"",
        0,
        "word  text\n1     [ls]\n2     [a.txt]\n3     [b.txt]\n4     [*.txt]\n5     [d/x.c]\nwords       5\n",
        0,
    );
}

#[test]
fn cli_quote_single_style_example_escapes_apostrophes() {
    assert_success(
        &["quote", "a", "b c", "it's", ""],
        b"",
        0,
        "a 'b c' 'it'\\''s' ''\n",
        0,
    );
}

#[test]
fn cli_quote_double_style_example_escapes_shell_characters() {
    assert_success(
        &["quote", "--style", "double", "say \"hi\" $HOME", "x"],
        b"",
        0,
        "\"say \\\"hi\\\" \\$HOME\" x\n",
        0,
    );
}

#[test]
fn cli_lint_example_orders_findings_and_returns_one() {
    assert_success(
        &["lint", "rm -rf $DIR/* \"$*\" ${X:-$Y} $@"],
        b"",
        1,
        "column 8: W1 unquoted parameter DIR is subject to splitting and pattern matching\ncolumn 16: W3 \"$*\" joins the parameters into one word; \"$@\" keeps them apart\ncolumn 20: W1 unquoted parameter X is subject to splitting and pattern matching\ncolumn 25: W1 unquoted parameter Y is subject to splitting and pattern matching\ncolumn 29: W2 unquoted $@ splits and matches its parameters again; write \"$@\"\nwarnings    5\n",
        0,
    );
}

#[test]
fn cli_empty_split_has_a_header_and_zero_words() {
    assert_success(&["split", ""], b"", 0, "word  parts\nwords       0\n", 0);
}

#[test]
fn cli_empty_braces_has_a_header_and_zero_words() {
    assert_success(&["braces", ""], b"", 0, "word  text\nwords       0\n", 0);
}

#[test]
fn cli_empty_fields_has_a_header_and_zero_fields() {
    assert_success(&["fields", ""], b"", 0, "field  text\nfields      0\n", 0);
}

#[test]
fn cli_empty_expand_has_a_header_and_zero_words() {
    assert_success(&["expand", ""], b"", 0, "word  text\nwords       0\n", 0);
}

#[test]
fn cli_lint_excludes_the_count_parameter() {
    assert_success(&["lint", "${#}"], b"", 0, "warnings    0\n", 0);
}

#[test]
fn cli_lint_returns_one_for_an_unquoted_parameter_finding() {
    assert_success(
        &["lint", "$X"],
        b"",
        1,
        "column 1: W1 unquoted parameter X is subject to splitting and pattern matching\nwarnings    1\n",
        0,
    );
}

#[test]
fn cli_glob_reports_a_missing_literal_match() {
    assert_success(
        &["glob", "z"],
        b"",
        0,
        "pattern     [z]\nmatch       -\n",
        0,
    );
}

#[test]
fn cli_fields_uses_the_last_ifs_option() {
    assert_success(
        &["fields", "--ifs", ",", "--ifs", ":", "a::"],
        b"",
        0,
        "field  text\n1      [a]\n2      []\nfields      2\n",
        0,
    );
}

#[test]
fn cli_quote_uses_the_last_style_option() {
    assert_success(
        &[
            "quote", "--style", "single", "--style", "double", "--always", "x",
        ],
        b"",
        0,
        "\"x\"\n",
        0,
    );
}

#[test]
fn cli_quote_accepts_a_repeated_always_flag() {
    assert_success(&["quote", "--always", "--always", "x"], b"", 0, "'x'\n", 0);
}

#[test]
fn cli_quote_treats_terminator_following_text_as_an_operand() {
    assert_success(&["quote", "--", "--always"], b"", 0, "--always\n", 0);
}

#[test]
fn cli_quote_preserves_unicode_and_embedded_crlf_payload() {
    assert_success(&["quote", "--always", "é\r\n"], b"", 0, "'é\r\n'\n", 0);
}

#[test]
fn cli_glob_matches_an_escaped_wildcard_as_a_literal() {
    assert_success(
        &["glob", "--file", "*", "\\*"],
        b"",
        0,
        "pattern     [\\*]\nmatch       [*]\n",
        0,
    );
}

#[test]
fn cli_expand_uses_the_last_repeated_variable_value() {
    assert_success(
        &["expand", "--var", "X=a", "--var", "X=b", "$X"],
        b"",
        0,
        "word  text\n1     [b]\nwords       1\n",
        0,
    );
}

#[test]
fn cli_quote_requires_text() {
    assert_error(
        &["quote"],
        b"",
        "shexpand: usage: shexpand quote [--style single|double] [--always] TEXT...\n",
    );
}

#[test]
fn cli_split_rejects_unknown_options() {
    assert_error(
        &["split", "--x", "a"],
        b"",
        "shexpand: unknown option \"--x\"\n",
    );
}

#[test]
fn cli_quote_rejects_a_value_for_the_always_flag() {
    assert_error(
        &["quote", "--always=1", "a"],
        b"",
        "shexpand: --always takes no value\n",
    );
}

#[test]
fn cli_expand_reports_a_missing_variable_option_value() {
    assert_error(&["expand", "--var"], b"", "shexpand: --var needs a value\n");
}

#[test]
fn cli_expand_rejects_a_variable_without_an_equals_sign() {
    assert_error(
        &["expand", "--var", "X", "echo"],
        b"",
        "shexpand: invalid --var \"X\" (expected NAME=VALUE with a name of letters, digits and _)\n",
    );
}

#[test]
fn cli_quote_rejects_an_unknown_style() {
    assert_error(
        &["quote", "--style", "round", "a"],
        b"",
        "shexpand: invalid --style \"round\" (expected single or double)\n",
    );
}

#[test]
fn cli_glob_rejects_empty_path_components() {
    assert_error(
        &["glob", "--file", "a//b", "a"],
        b"",
        "shexpand: invalid --file \"a//b\" (expected a path of names separated by single slashes)\n",
    );
}

#[test]
fn cli_glob_rejects_a_path_that_is_both_a_file_and_directory() {
    assert_error(
        &["glob", "--file", "a", "--file", "a/b", "a"],
        b"",
        "shexpand: \"a/b\" is both a file and a directory\n",
    );
}

#[test]
fn cli_split_reports_unterminated_single_quotes() {
    assert_error(
        &["split", "echo 'a"],
        b"",
        "shexpand: unterminated single quote starting at column 6\n",
    );
}

#[test]
fn cli_split_reports_unterminated_double_quotes() {
    assert_error(
        &["split", "echo \"a"],
        b"",
        "shexpand: unterminated double quote starting at column 6\n",
    );
}

#[test]
fn cli_split_reports_unterminated_parameter_braces() {
    assert_error(
        &["split", "echo ${A:-b"],
        b"",
        "shexpand: unterminated ${ starting at column 6\n",
    );
}

#[test]
fn cli_split_reports_a_trailing_backslash() {
    assert_error(
        &["split", "echo a\\"],
        b"",
        "shexpand: trailing backslash at column 7\n",
    );
}

#[test]
fn cli_split_reports_unsupported_operators() {
    assert_error(
        &["split", "a | b"],
        b"",
        "shexpand: unsupported operator \"|\" at column 3\n",
    );
}

#[test]
fn cli_split_reports_unescaped_command_substitution() {
    assert_error(
        &["split", "echo `ls`"],
        b"",
        "shexpand: command substitution is not supported at column 6\n",
    );
}

#[test]
fn cli_split_reports_unsupported_special_parameters() {
    assert_error(
        &["split", "echo $?"],
        b"",
        "shexpand: unsupported parameter \"$?\" at column 6\n",
    );
}

#[test]
fn cli_split_reports_unsupported_dollar_quoting() {
    assert_error(
        &["split", "echo $\"a\""],
        b"",
        "shexpand: unsupported quoting after $ at column 6\n",
    );
}

#[test]
fn cli_split_reports_bad_parameter_substitutions() {
    assert_error(
        &["split", "echo ${X^}"],
        b"",
        "shexpand: bad substitution \"${X^}\" at column 6\n",
    );
}

#[test]
fn cli_split_rejects_double_quotes_inside_double_quoted_operator_words() {
    assert_error(
        &["split", "echo \"${X:-\"a\"}\""],
        b"",
        "shexpand: unsupported double quote inside \"${...}\" at column 12\n",
    );
}

#[test]
fn cli_split_reports_parameter_nesting_beyond_eight() {
    assert_error(
        &[
            "split",
            "${A:-${A:-${A:-${A:-${A:-${A:-${A:-${A:-${A:-x}}}}}}}}}",
        ],
        b"",
        "shexpand: nesting is deeper than 8 at column 41\n",
    );
}

#[test]
fn cli_braces_reports_nesting_beyond_eight() {
    assert_error(
        &["braces", "{a,{a,{a,{a,{a,{a,{a,{a,{a,b}}}}}}}}}"],
        b"",
        "shexpand: nesting is deeper than 8\n",
    );
}

#[test]
fn cli_braces_reports_more_than_one_thousand_expansions() {
    assert_error(
        &["braces", "{1..2000}"],
        b"",
        "shexpand: brace expansion makes more than 1000 words\n",
    );
}

#[test]
fn cli_expand_reports_a_custom_parameter_error_message() {
    assert_error(&["expand", "echo ${Z:?oops}"], b"", "shexpand: Z: oops\n");
}

#[test]
fn cli_expand_reports_the_colon_error_for_an_empty_parameter() {
    assert_error(
        &["expand", "echo ${Z:?}"],
        b"",
        "shexpand: Z: parameter null or not set\n",
    );
}

#[test]
fn cli_expand_reports_the_non_colon_error_for_an_unset_parameter() {
    assert_error(
        &["expand", "echo ${Z?}"],
        b"",
        "shexpand: Z: parameter not set\n",
    );
}

#[test]
fn cli_expand_rejects_assignment_to_a_positional_parameter() {
    assert_error(
        &["expand", "echo ${1:=x}"],
        b"",
        "shexpand: 1: cannot assign in this way\n",
    );
}

#[test]
fn cli_split_checks_options_before_arity() {
    assert_error(&["split", "--x"], b"", "shexpand: unknown option \"--x\"\n");
}

#[test]
fn cli_quote_checks_style_before_arity() {
    assert_error(
        &["quote", "--style", "round"],
        b"",
        "shexpand: invalid --style \"round\" (expected single or double)\n",
    );
}

#[test]
fn cli_expand_validates_variables_before_files_and_stdin() {
    assert_error(
        &["expand", "--file", "a//b", "--var", "X", "-"],
        b"",
        "shexpand: invalid --var \"X\" (expected NAME=VALUE with a name of letters, digits and _)\n",
    );
}

#[test]
fn cli_expand_checks_arity_before_constructing_virtual_files() {
    assert_error(
        &["expand", "--file", "a//b"],
        b"",
        "shexpand: usage: shexpand expand [--var NAME=VALUE]... [--ifs TEXT] [--file PATH]... LINE [ARG...]\n",
    );
}

#[test]
fn cli_expand_validates_files_before_reading_stdin() {
    assert_error(
        &["expand", "--file", "a//b", "-"],
        b"",
        "shexpand: invalid --file \"a//b\" (expected a path of names separated by single slashes)\n",
    );
}

#[test]
fn cli_glob_checks_arity_before_constructing_virtual_files() {
    assert_error(
        &["glob", "--file", "a//b"],
        b"",
        "shexpand: usage: shexpand glob [--file PATH]... PATTERN...\n",
    );
}

#[test]
fn cli_quote_reports_the_first_invalid_repeated_style() {
    assert_error(
        &["quote", "--style", "round", "--style", "single", "a"],
        b"",
        "shexpand: invalid --style \"round\" (expected single or double)\n",
    );
}

#[test]
fn cli_split_checks_arity_before_reading_stdin() {
    assert_error(
        &["split", "-", "extra"],
        b"",
        "shexpand: usage: shexpand split LINE\n",
    );
}

#[test]
fn cli_split_does_not_write_partial_output_after_a_later_lex_error() {
    assert_error(
        &["split", "ok '"],
        b"",
        "shexpand: unterminated single quote starting at column 4\n",
    );
}

#[test]
fn cli_braces_does_not_write_partial_output_after_a_later_expansion_error() {
    assert_error(
        &["braces", "ok {1..2000}"],
        b"",
        "shexpand: brace expansion makes more than 1000 words\n",
    );
}

#[test]
fn cli_expand_does_not_write_assignment_rows_after_a_later_parameter_error() {
    assert_error(
        &["expand", "ok ${N:=7} ${Z:?oops}"],
        b"",
        "shexpand: Z: oops\n",
    );
}

#[test]
fn cli_split_reads_bom_prefixed_crlf_stdin_once() {
    assert_success(
        &["split", "-"],
        b"\xef\xbb\xbfa\r\n",
        0,
        "word  parts\n1     u[a]\nwords       1\n",
        1,
    );
}

#[test]
fn cli_split_reports_malformed_utf8_after_one_stdin_read() {
    let io = RecordingIo::new(&[0x65, 0x63, 0xFF, 0x0A]);

    assert_eq!(shexpand::cli::run(&owned_args(&["split", "-"]), &io), 2);
    assert_eq!(io.reads.get(), 1);
    assert!(io.outputs.borrow().is_empty());
    assert_eq!(
        io.errors.borrow().as_slice(),
        &["shexpand: <stdin>: input is not valid UTF-8\n".to_owned()]
    );
}

#[test]
fn cli_split_does_not_read_stdin_for_a_literal_operand() {
    assert_success(
        &["split", "a"],
        &[0xFF],
        0,
        "word  parts\n1     u[a]\nwords       1\n",
        0,
    );
}

#[test]
fn cli_quote_treats_dash_as_literal_text_without_reading_stdin() {
    assert_success(&["quote", "-"], b"", 0, "-\n", 0);
}

#[test]
fn cli_glob_treats_dash_as_a_literal_pattern_without_reading_stdin() {
    assert_success(
        &["glob", "-"],
        b"",
        0,
        "pattern     [-]\nmatch       -\n",
        0,
    );
}

#[test]
fn cli_braces_reads_accepted_stdin_once() {
    assert_success(
        &["braces", "-"],
        b"a{b,c}\r\n",
        0,
        "word  text\n1     [ab]\n1     [ac]\nwords       2\n",
        1,
    );
}

#[test]
fn cli_fields_reads_accepted_stdin_once() {
    assert_success(
        &["fields", "--ifs", ":", "-"],
        b"a:b\n",
        0,
        "field  text\n1      [a]\n2      [b]\nfields      2\n",
        1,
    );
}

#[test]
fn cli_expand_reads_accepted_stdin_once() {
    assert_success(
        &["expand", "-"],
        b"echo hi\n",
        0,
        "word  text\n1     [echo]\n2     [hi]\nwords       2\n",
        1,
    );
}

#[test]
fn cli_lint_reads_accepted_stdin_once() {
    assert_success(
        &["lint", "-"],
        b"$X\n",
        1,
        "column 1: W1 unquoted parameter X is subject to splitting and pattern matching\nwarnings    1\n",
        1,
    );
}

#[test]
fn process_split_rejects_malformed_utf8_stdin_with_exact_streams() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_shexpand"))
        .args(["split", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the shexpand test binary should start");
    let mut stdin = child.stdin.take().expect("stdin should be piped");
    stdin
        .write_all(&[0x65, 0x63, 0xFF, 0x0A])
        .expect("the test bytes should be written");
    drop(stdin);

    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the shexpand process exceeded the 30-second timeout");
            }
            Err(error) => panic!("could not wait for the shexpand process: {error}"),
        }
    };
    let output = child
        .wait_with_output()
        .expect("the shexpand process output should be collected");

    assert_eq!(status.code(), Some(2));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        output.stderr,
        b"shexpand: <stdin>: input is not valid UTF-8\n"
    );
}
