use shexpand::atom::Atom;
use shexpand::cmd_quote::{USAGE, run};
use shexpand::error::ShError;
use shexpand::io::Io;
use shexpand::lexer::lex;
use shexpand::quoter::quote;
use shexpand::render::atom_text;
use std::cell::{Cell, RefCell};

fn assert_literal_round_trip(input: &str, quoted: &str) {
    let words = lex(quoted).expect("quoted text should be valid shell syntax");
    assert_eq!(words.len(), 1);
    assert!(
        !words[0]
            .atoms
            .iter()
            .any(|atom| matches!(atom, Atom::Param(_)))
    );
    assert_eq!(atom_text(&words[0].atoms), input);
}

fn assert_both_style_round_trips(input: &str, single: &str, double: &str) {
    let single_result = quote(input, false, false);
    assert_eq!(single_result, single);
    assert_literal_round_trip(input, &single_result);

    let double_result = quote(input, true, false);
    assert_eq!(double_result, double);
    assert_literal_round_trip(input, &double_result);
}

#[test]
fn quote_leaves_the_ascii_safe_alphabet_unquoted() {
    assert_eq!(quote("Az09_@%+=:,./-", false, false), "Az09_@%+=:,./-");
}

#[test]
fn quote_wraps_empty_single_style_text() {
    assert_eq!(quote("", false, false), "''");
}

#[test]
fn quote_single_quotes_whitespace() {
    assert_eq!(quote("a b", false, false), "'a b'");
}

#[test]
fn quote_single_quotes_unsafe_punctuation() {
    assert_eq!(quote("a#b", false, false), "'a#b'");
}

#[test]
fn quote_single_quotes_shell_metacharacters() {
    assert_eq!(quote("*?[]{}~!;|&<>()", false, false), "'*?[]{}~!;|&<>()'");
}

#[test]
fn quote_single_quotes_unicode() {
    assert_eq!(quote("é", false, false), "'é'");
}

#[test]
fn quote_escapes_apostrophes_in_single_style() {
    assert_eq!(quote("it's", false, false), "'it'\\''s'");
}

#[test]
fn quote_always_wraps_single_style_text() {
    assert_eq!(quote("x", false, true), "'x'");
}

#[test]
fn quote_always_wraps_double_style_text() {
    assert_eq!(quote("x", true, true), "\"x\"");
}

#[test]
fn quote_leaves_safe_double_style_text_unquoted() {
    assert_eq!(quote("x", true, false), "x");
}

#[test]
fn quote_wraps_empty_double_style_text() {
    assert_eq!(quote("", true, false), "\"\"");
}

#[test]
fn quote_escapes_all_four_double_style_metacharacters() {
    assert_eq!(quote("$`\"\\", true, false), "\"\\$\\`\\\"\\\\\"");
}

#[test]
fn quote_round_trips_empty_text_in_both_styles() {
    assert_both_style_round_trips("", "''", "\"\"");
}

#[test]
fn quote_round_trips_apostrophes_in_both_styles() {
    assert_both_style_round_trips("it's", "'it'\\''s'", "\"it's\"");
}

#[test]
fn quote_round_trips_dollar_text_in_both_styles() {
    assert_both_style_round_trips("$HOME", "'$HOME'", "\"\\$HOME\"");
}

#[test]
fn quote_round_trips_unicode_in_both_styles() {
    assert_both_style_round_trips("é", "'é'", "\"é\"");
}

#[test]
fn quote_round_trips_embedded_line_feed_in_both_styles() {
    assert_both_style_round_trips("a\nb", "'a\nb'", "\"a\nb\"");
}

#[test]
fn quote_round_trips_embedded_carriage_return_in_both_styles() {
    assert_both_style_round_trips("a\rb", "'a\rb'", "\"a\rb\"");
}

#[test]
fn quote_round_trips_embedded_crlf_in_both_styles() {
    assert_both_style_round_trips("a\r\nb", "'a\r\nb'", "\"a\r\nb\"");
}

struct QuoteIo {
    reads: Cell<usize>,
    outputs: RefCell<Vec<String>>,
    errors: RefCell<Vec<String>>,
}

impl QuoteIo {
    fn new() -> Self {
        Self {
            reads: Cell::new(0),
            outputs: RefCell::new(Vec::new()),
            errors: RefCell::new(Vec::new()),
        }
    }
}

impl Io for QuoteIo {
    fn read_stdin(&self) -> Vec<u8> {
        self.reads.set(self.reads.get() + 1);
        Vec::new()
    }

    fn out(&self, text: &str) {
        self.outputs.borrow_mut().push(text.to_owned());
    }

    fn err(&self, text: &str) {
        self.errors.borrow_mut().push(text.to_owned());
    }
}

fn string_args(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_owned()).collect()
}

fn assert_quote_success(args: &[&str], expected: &str) {
    let io = QuoteIo::new();

    assert_eq!(run(&string_args(args), &io), Ok(0));
    assert_eq!(*io.outputs.borrow(), vec![expected.to_owned()]);
    assert!(io.errors.borrow().is_empty());
    assert_eq!(io.reads.get(), 0);
}

fn assert_quote_error(args: &[&str], expected: &str) {
    let io = QuoteIo::new();

    assert_eq!(
        run(&string_args(args), &io),
        Err(ShError(expected.to_owned()))
    );
    assert!(io.outputs.borrow().is_empty());
    assert!(io.errors.borrow().is_empty());
    assert_eq!(io.reads.get(), 0);
}

#[test]
fn quote_adapter_exposes_the_exact_usage() {
    assert_eq!(
        USAGE,
        "shexpand quote [--style single|double] [--always] TEXT..."
    );
}

#[test]
fn quote_adapter_quotes_default_operands_in_order_with_one_output_call() {
    assert_quote_success(&["a", "b c", "it's", ""], "a 'b c' 'it'\\''s' ''\n");
}

#[test]
fn quote_adapter_applies_double_style_to_all_operands() {
    assert_quote_success(
        &["--style", "double", "say \"hi\" $HOME", "x"],
        "\"say \\\"hi\\\" \\$HOME\" x\n",
    );
}

#[test]
fn quote_adapter_uses_repeated_style_and_always_options_in_order() {
    assert_quote_success(
        &[
            "x",
            "--always",
            "--always",
            "--style=single",
            "--style=double",
            "y",
        ],
        "\"x\" \"y\"\n",
    );
}

#[test]
fn quote_adapter_treats_post_terminator_options_and_dash_as_operands() {
    assert_quote_success(&["--", "--style", "-"], "--style -\n");
}

#[test]
fn quote_adapter_preserves_unicode_and_crlf_payloads() {
    assert_quote_success(&["é", "a\r\nb"], "'é' 'a\r\nb'\n");
}

#[test]
fn quote_adapter_reports_usage_for_missing_operands_without_io() {
    assert_quote_error(
        &[],
        "usage: shexpand quote [--style single|double] [--always] TEXT...",
    );
}

#[test]
fn quote_adapter_validates_style_before_arity() {
    assert_quote_error(
        &["--style", "round"],
        "invalid --style \"round\" (expected single or double)",
    );
}

#[test]
fn quote_adapter_sanitizes_invalid_style_diagnostics() {
    assert_quote_error(
        &["--style", "é\n", "a"],
        "invalid --style \"??\" (expected single or double)",
    );
}

#[test]
fn quote_adapter_rejects_an_invalid_earlier_repeated_style() {
    assert_quote_error(
        &["--style=round", "--style=single", "a"],
        "invalid --style \"round\" (expected single or double)",
    );
}

#[test]
fn quote_adapter_rejects_values_for_the_always_flag() {
    assert_quote_error(&["--always=1", "a"], "--always takes no value");
}

#[test]
fn quote_adapter_reports_a_missing_style_value() {
    assert_quote_error(&["--style"], "--style needs a value");
}

#[test]
fn quote_adapter_parses_all_options_before_style_validation() {
    assert_quote_error(&["--style=round", "--x", "a"], "unknown option \"--x\"");
}
