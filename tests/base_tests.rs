use shexpand::atom::{Atom, Kind, Param, Word};
use shexpand::error::ShError;
use shexpand::io::{Io, text_arg};
use shexpand::options::{Parsed, Spec, parse};
use shexpand::text::{cell, line, show, table};
use std::cell::{Cell as ReadCount, RefCell};
use std::collections::BTreeMap;

fn specs() -> [Spec; 3] {
    [
        Spec {
            name: "--var",
            flag: false,
        },
        Spec {
            name: "--ifs",
            flag: false,
        },
        Spec {
            name: "--always",
            flag: true,
        },
    ]
}

#[test]
fn atom_variants_can_be_constructed_and_compared() {
    let plain = Atom::Ch {
        ch: 'x',
        quoted: true,
    };
    let marker = Atom::Quote;
    let parameter = Atom::Param(Box::new(Param {
        kind: Kind::Plain,
        name: "X".to_owned(),
        colon: false,
        longest: false,
        word: vec![],
        text: "$X".to_owned(),
        col: 1,
        quoted: false,
    }));

    assert_eq!(
        plain,
        Atom::Ch {
            ch: 'x',
            quoted: true
        }
    );
    assert_eq!(marker, Atom::Quote);
    assert_ne!(
        plain,
        Atom::Ch {
            ch: 'x',
            quoted: false
        }
    );
    assert!(matches!(parameter, Atom::Param(_)));
}

#[test]
fn param_retains_each_public_field_and_clone_compares_equal() {
    let param = Param {
        kind: Kind::Default,
        name: "X".to_owned(),
        colon: true,
        longest: false,
        word: vec![Atom::Ch {
            ch: 'a',
            quoted: false,
        }],
        text: r"${X:-a}".to_owned(),
        col: 7,
        quoted: true,
    };
    let clone = param.clone();

    assert_eq!(clone, param);
    assert_eq!(param.kind, Kind::Default);
    assert_eq!(param.name, "X");
    assert!(param.colon);
    assert!(!param.longest);
    assert_eq!(
        param.word,
        vec![Atom::Ch {
            ch: 'a',
            quoted: false
        }]
    );
    assert_eq!(param.text, r"${X:-a}");
    assert!(format!("{param:?}").contains("Param"));
    assert_eq!(param.col, 7);
    assert!(param.quoted);
}

#[test]
fn all_parameter_kinds_are_public_and_distinct() {
    let kinds = vec![
        Kind::Plain,
        Kind::Length,
        Kind::Default,
        Kind::Assign,
        Kind::Alt,
        Kind::Error,
        Kind::Prefix,
        Kind::Suffix,
    ];

    assert_eq!(
        kinds,
        vec![
            Kind::Plain,
            Kind::Length,
            Kind::Default,
            Kind::Assign,
            Kind::Alt,
            Kind::Error,
            Kind::Prefix,
            Kind::Suffix,
        ]
    );
    assert_ne!(Kind::Prefix, Kind::Suffix);
}

#[test]
fn word_atoms_are_public_and_debug_is_available() {
    let word = Word {
        atoms: vec![
            Atom::Quote,
            Atom::Ch {
                ch: 'z',
                quoted: false,
            },
        ],
    };

    assert_eq!(word.atoms.len(), 2);
    assert_eq!(
        format!("{word:?}"),
        "Word { atoms: [Quote, Ch { ch: 'z', quoted: false }] }"
    );
    assert!(format!("{:?}", ShError("oops".to_owned())).contains("ShError"));
}

#[test]
fn sh_error_owns_its_message_and_implements_eq() {
    let error = ShError("oops".to_owned());

    assert_eq!(ShError("oops".into()).0, "oops");
    assert_eq!(error.0, "oops");
    assert_eq!(error, ShError("oops".to_owned()));
    assert_ne!(error, ShError("other".to_owned()));
}

#[test]
fn show_wraps_empty_text() {
    assert_eq!(show(""), "\"\"");
}

#[test]
fn cell_wraps_empty_text() {
    assert_eq!(cell(""), "[]");
}

#[test]
fn show_preserves_printable_ascii_boundaries() {
    assert_eq!(show(" ~"), "\" ~\"");
    assert_eq!(show("!~"), "\"!~\"");
}

#[test]
fn show_replaces_each_non_printable_character_once() {
    assert_eq!(show("é\t"), r#""??""#);
}

#[test]
fn show_keeps_quote_characters_as_printable_text() {
    assert_eq!(show("\""), r#"""""#);
}

#[test]
fn cell_replaces_each_non_printable_character_once() {
    assert_eq!(cell("a\t\né😀\u{7f}"), "[a?????]");
}

#[test]
fn cell_preserves_printable_punctuation() {
    assert_eq!(cell("\"\\[]"), "[\"\\[]]");
}

#[test]
fn show_preserves_printable_punctuation() {
    assert_eq!(
        show("!#$%&'()*+,-./:;<=>?@[\\]^_\u{60}{|}~"),
        "\"!#$%&'()*+,-./:;<=>?@[\\]^_\u{60}{|}~\""
    );
}

#[test]
fn line_pads_short_keys_to_ten_characters() {
    assert_eq!(line("words", "4"), "words       4");
}

#[test]
fn line_does_not_truncate_long_keys() {
    assert_eq!(line("abcdefghijk", "x"), "abcdefghijk  x");
}

#[test]
fn line_counts_unicode_characters_not_bytes() {
    assert_eq!(line("é", "x"), "é           x");
}

#[test]
fn line_does_not_append_a_line_ending() {
    assert_eq!(line("a", "b"), "a           b");
}

#[test]
fn table_aligns_rectangular_rows_using_column_maxima() {
    assert_eq!(
        table(
            &["word", "parts"],
            &[
                vec!["1".into(), "u[cp]".into()],
                vec!["12".into(), "q[x y]".into()]
            ]
        ),
        vec!["word  parts", "1     u[cp]", "12    q[x y]"]
    );
}

#[test]
fn table_includes_header_width_and_trims_trailing_spaces() {
    assert_eq!(
        table(&["a", "long"], &[vec!["wide".into(), "".into()]]),
        vec!["a     long", "wide"]
    );
}

#[test]
fn table_fills_missing_ragged_cells_with_empty_text() {
    assert_eq!(
        table(&["a"], &[vec!["x".into(), "y".into()], vec![]]),
        vec!["a", "x  y", ""]
    );
}

#[test]
fn table_keeps_one_empty_line_for_zero_columns() {
    assert_eq!(table(&[], &[]), vec![""]);
}

#[test]
fn table_counts_unicode_cell_width_in_characters() {
    assert_eq!(
        table(&["é", "x"], &[vec!["a".into(), "éé".into()]]),
        vec!["é  x", "a  éé"]
    );
}

#[test]
fn options_initialize_an_empty_value_list_for_every_spec() {
    let parsed = parse(&[], &specs()).unwrap();

    assert_eq!(
        parsed,
        Parsed {
            values: BTreeMap::from([
                ("--always".to_owned(), vec![]),
                ("--ifs".to_owned(), vec![]),
                ("--var".to_owned(), vec![]),
            ]),
            args: vec![],
        }
    );
}

#[test]
fn options_preserve_order_repetitions_and_ordinary_arguments() {
    let parsed = parse(
        &[
            "a".to_owned(),
            "--var=X=1".to_owned(),
            "-".to_owned(),
            "--always".to_owned(),
            "--var".to_owned(),
            "-x".to_owned(),
            "--always".to_owned(),
            "-x".to_owned(),
        ],
        &specs(),
    )
    .unwrap();

    assert_eq!(
        parsed.values,
        BTreeMap::from([
            ("--always".to_owned(), vec!["".to_owned(), "".to_owned()]),
            ("--ifs".to_owned(), vec![]),
            ("--var".to_owned(), vec!["X=1".to_owned(), "-x".to_owned()]),
        ])
    );
    assert_eq!(parsed.args, vec!["a", "-", "-x"]);
}

#[test]
fn options_allow_empty_inline_values_and_drop_the_terminator() {
    let parsed = parse(
        &[
            "--var=".to_owned(),
            "--".to_owned(),
            "--always".to_owned(),
            "--x".to_owned(),
        ],
        &specs(),
    )
    .unwrap();

    assert_eq!(
        parsed.values,
        BTreeMap::from([
            ("--always".to_owned(), vec![]),
            ("--ifs".to_owned(), vec![]),
            ("--var".to_owned(), vec!["".to_owned()]),
        ])
    );
    assert_eq!(parsed.args, vec!["--always", "--x"]);
}

#[test]
fn valued_option_consumes_a_terminator_as_its_value() {
    let parsed = parse(
        &["--var".to_owned(), "--".to_owned(), "--always".to_owned()],
        &specs(),
    )
    .unwrap();

    assert_eq!(
        parsed.values,
        BTreeMap::from([
            ("--always".to_owned(), vec!["".to_owned()]),
            ("--ifs".to_owned(), vec![]),
            ("--var".to_owned(), vec!["--".to_owned()]),
        ])
    );
    assert_eq!(parsed.args, Vec::<String>::new());
}

fn option_error(argv: Vec<String>) -> ShError {
    parse(&argv, &specs()).unwrap_err()
}

#[test]
fn options_report_unknown_name_with_the_whole_argument_sanitized() {
    assert_eq!(
        option_error(vec!["--x=a".to_owned()]),
        ShError("unknown option \"--x=a\"".to_owned())
    );
}

#[test]
fn options_sanitize_non_ascii_in_unknown_name() {
    assert_eq!(
        option_error(vec!["--é".to_owned()]),
        ShError("unknown option \"--?\"".to_owned())
    );
}

#[test]
fn flag_option_rejects_inline_value() {
    assert_eq!(
        option_error(vec!["--always=1".to_owned()]),
        ShError("--always takes no value".to_owned())
    );
}

#[test]
fn flag_option_rejects_empty_inline_value() {
    assert_eq!(
        option_error(vec!["--always=".to_owned()]),
        ShError("--always takes no value".to_owned())
    );
}

#[test]
fn valued_option_reports_missing_value() {
    assert_eq!(
        option_error(vec!["--var".to_owned()]),
        ShError("--var needs a value".to_owned())
    );
}

#[test]
fn options_return_unknown_name_as_the_first_error() {
    assert_eq!(
        option_error(vec!["--x".to_owned(), "--always=1".to_owned()]),
        ShError("unknown option \"--x\"".to_owned())
    );
}

#[test]
fn options_return_flag_value_error_as_the_first_error() {
    assert_eq!(
        option_error(vec!["--always=1".to_owned(), "--x".to_owned()]),
        ShError("--always takes no value".to_owned())
    );
}

struct FakeIo {
    input: Vec<u8>,
    reads: ReadCount<usize>,
    outputs: RefCell<Vec<String>>,
    errors: RefCell<Vec<String>>,
}

impl FakeIo {
    fn new(input: Vec<u8>) -> Self {
        Self {
            input,
            reads: ReadCount::new(0),
            outputs: RefCell::new(Vec::new()),
            errors: RefCell::new(Vec::new()),
        }
    }
}

impl Io for FakeIo {
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

fn assert_io_quiet(io: &FakeIo, reads: usize) {
    assert_eq!(io.reads.get(), reads);
    assert!(io.outputs.borrow().is_empty());
    assert!(io.errors.borrow().is_empty());
}

#[test]
fn text_arg_returns_literal_text_without_reading_stdin() {
    let arg = "é\r\n";
    let stdin = vec![0xff];
    let io = FakeIo::new(stdin);

    assert_eq!(text_arg(&io, arg), Ok("é\r\n".to_owned()));
    assert_io_quiet(&io, 0);
}

#[test]
fn text_arg_rejects_malformed_utf8() {
    let arg = "-";
    let stdin = vec![0x65, 0x63, 0xff, 0x0a];
    let io = FakeIo::new(stdin);

    assert_eq!(
        text_arg(&io, arg),
        Err(ShError("<stdin>: input is not valid UTF-8".to_owned()))
    );
    assert_io_quiet(&io, 1);
}

#[test]
fn text_arg_rejects_incomplete_utf8() {
    let arg = "-";
    let stdin = vec![0xc3];
    let io = FakeIo::new(stdin);

    assert_eq!(
        text_arg(&io, arg),
        Err(ShError("<stdin>: input is not valid UTF-8".to_owned()))
    );
    assert_io_quiet(&io, 1);
}

#[test]
fn text_arg_accepts_multibyte_utf8() {
    let arg = "-";
    let stdin = vec![0xc3, 0xa9];
    let io = FakeIo::new(stdin);

    assert_eq!(text_arg(&io, arg), Ok("é".to_owned()));
    assert_io_quiet(&io, 1);
}

#[test]
fn text_arg_accepts_empty_stdin() {
    let io = FakeIo::new(vec![]);

    assert_eq!(text_arg(&io, "-").unwrap(), "");
    assert_io_quiet(&io, 1);
}

#[test]
fn text_arg_removes_one_leading_bom_and_crlf() {
    let io = FakeIo::new(vec![0xef, 0xbb, 0xbf, 0x61, 0x0d, 0x0a]);

    assert_eq!(text_arg(&io, "-").unwrap(), "a");
    assert_io_quiet(&io, 1);
}

#[test]
fn text_arg_removes_only_one_leading_bom() {
    let io = FakeIo::new(vec![0xef, 0xbb, 0xbf, 0xef, 0xbb, 0xbf, 0x61]);

    assert_eq!(text_arg(&io, "-").unwrap(), "\u{feff}a");
    assert_io_quiet(&io, 1);
}

#[test]
fn text_arg_removes_only_one_final_lf() {
    let io = FakeIo::new(vec![0x61, 0x0a, 0x0a]);

    assert_eq!(text_arg(&io, "-").unwrap(), "a\n");
    assert_io_quiet(&io, 1);
}

#[test]
fn text_arg_removes_only_one_final_crlf() {
    let io = FakeIo::new(vec![0x61, 0x0d, 0x0a, 0x0d, 0x0a]);

    assert_eq!(text_arg(&io, "-").unwrap(), "a\r\n");
    assert_io_quiet(&io, 1);
}

#[test]
fn text_arg_preserves_standalone_carriage_return() {
    let io = FakeIo::new(vec![0x61, 0x0d]);

    assert_eq!(text_arg(&io, "-").unwrap(), "a\r");
    assert_io_quiet(&io, 1);
}

#[test]
fn text_arg_preserves_internal_line_endings() {
    let io = FakeIo::new(vec![0x61, 0x0d, 0x0a, 0x62, 0x0a]);

    assert_eq!(text_arg(&io, "-").unwrap(), "a\r\nb");
    assert_io_quiet(&io, 1);
}

#[test]
fn text_arg_preserves_a_nonleading_bom() {
    let io = FakeIo::new(vec![0x61, 0xef, 0xbb, 0xbf]);

    assert_eq!(text_arg(&io, "-").unwrap(), "a\u{feff}");
    assert_io_quiet(&io, 1);
}
