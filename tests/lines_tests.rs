use shexpand::cmd_quote::run;
use shexpand::error::ShError;
use shexpand::io::Io;
use std::cell::{Cell, RefCell};

struct LinesIo {
    reads: Cell<usize>,
    outputs: RefCell<Vec<String>>,
    errors: RefCell<Vec<String>>,
}

impl LinesIo {
    fn new() -> Self {
        Self {
            reads: Cell::new(0),
            outputs: RefCell::new(Vec::new()),
            errors: RefCell::new(Vec::new()),
        }
    }
}

impl Io for LinesIo {
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

fn assert_success(args: &[&str], expected: &str) {
    let io = LinesIo::new();

    assert_eq!(run(&string_args(args), &io), Ok(0));
    assert_eq!(*io.outputs.borrow(), vec![expected.to_owned()]);
    assert!(io.errors.borrow().is_empty());
    assert_eq!(io.reads.get(), 0);
}

fn assert_error(args: &[&str], expected: &str) {
    let io = LinesIo::new();

    assert_eq!(
        run(&string_args(args), &io),
        Err(ShError(expected.to_owned()))
    );
    assert!(io.outputs.borrow().is_empty());
    assert!(io.errors.borrow().is_empty());
    assert_eq!(io.reads.get(), 0);
}

#[test]
fn lines_writes_one_quoted_word_per_line_in_one_output_call() {
    assert_success(
        &["--lines", "a", "b c", "it's", ""],
        "a\n'b c'\n'it'\\''s'\n''\n",
    );
}

#[test]
fn lines_applies_the_style_and_always_to_every_line() {
    assert_success(
        &["--lines", "--style", "double", "--always", "x", "say $HOME"],
        "\"x\"\n\"say \\$HOME\"\n",
    );
}

#[test]
fn lines_may_come_after_the_operands_and_be_repeated() {
    assert_success(&["a", "--lines", "--lines", "b"], "a\nb\n");
}

#[test]
fn lines_with_one_operand_ends_with_one_line_feed() {
    assert_success(&["--lines", "only"], "only\n");
}

#[test]
fn lines_after_the_terminator_is_an_operand() {
    assert_success(&["--lines", "--", "--lines"], "--lines\n");
}

#[test]
fn quote_without_lines_still_joins_the_words_with_spaces() {
    assert_success(&["a", "b c"], "a 'b c'\n");
}

#[test]
fn lines_takes_no_value() {
    assert_error(&["--lines=1", "a"], "--lines takes no value");
}

#[test]
fn lines_without_operands_reports_the_usage_with_the_option() {
    assert_error(
        &["--lines"],
        "usage: shexpand quote [--style single|double] [--always] [--lines] TEXT...",
    );
}
