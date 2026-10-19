use shexpand::atom::{Atom, Kind, Param, Word};
use shexpand::lexer::lex;
use shexpand::linter::{Finding, lint};

fn parameter(kind: Kind, name: &str, col: usize, quoted: bool, word: Vec<Atom>) -> Atom {
    Atom::Param(Box::new(Param {
        kind,
        name: name.to_owned(),
        colon: false,
        longest: false,
        word,
        text: format!("${name}"),
        col,
        quoted,
    }))
}

fn finding(col: usize, code: &'static str, message: &str) -> Finding {
    Finding {
        col,
        code,
        message: message.to_owned(),
    }
}

#[test]
fn lint_reports_ordinary_parameters_at_their_source_columns() {
    let words = lex(r#"$X ${HOME} $1 ${99}"#).unwrap();
    let original = words.clone();

    assert_eq!(
        lint(&words),
        vec![
            finding(
                1,
                "W1",
                "unquoted parameter X is subject to splitting and pattern matching",
            ),
            finding(
                4,
                "W1",
                "unquoted parameter HOME is subject to splitting and pattern matching",
            ),
            finding(
                12,
                "W1",
                "unquoted parameter 1 is subject to splitting and pattern matching",
            ),
            finding(
                15,
                "W1",
                "unquoted parameter 99 is subject to splitting and pattern matching",
            ),
        ],
    );
    assert_eq!(words, original);
}

#[test]
fn lint_distinguishes_unquoted_and_quoted_positional_parameters() {
    let words = lex(r#"$@ $* "$*""#).unwrap();

    assert_eq!(
        lint(&words),
        vec![
            finding(
                1,
                "W2",
                r#"unquoted $@ splits and matches its parameters again; write "$@""#,
            ),
            finding(
                4,
                "W2",
                r#"unquoted $* splits and matches its parameters again; write "$@""#,
            ),
            finding(
                8,
                "W3",
                r#""$*" joins the parameters into one word; "$@" keeps them apart"#,
            ),
        ],
    );
}

#[test]
fn lint_excludes_special_and_length_parameters_and_quoted_values() {
    let words = lex(r#"$# ${#} ${#X} "$X" "$@" '$X' \$X"#).unwrap();

    assert_eq!(lint(&words), Vec::<Finding>::new());
}

#[test]
fn lint_ignores_literals_and_text_after_a_word_starting_comment() {
    let words = lex(r#"plain "" # $X"#).unwrap();

    assert_eq!(lint(&words), Vec::<Finding>::new());
}

#[test]
fn lint_returns_no_findings_for_empty_input() {
    assert_eq!(lint(&[]), Vec::<Finding>::new());
}

#[test]
fn lint_recurses_into_an_unquoted_default_operator_word() {
    let words = lex(r#"${X:-$Y}"#).unwrap();

    assert_eq!(
        lint(&words),
        vec![
            finding(
                1,
                "W1",
                "unquoted parameter X is subject to splitting and pattern matching",
            ),
            finding(
                6,
                "W1",
                "unquoted parameter Y is subject to splitting and pattern matching",
            ),
        ],
    );
}

#[test]
fn lint_suppresses_all_findings_inside_a_quoted_operator() {
    let words = lex(r#""${X:-$Y}""#).unwrap();

    assert_eq!(lint(&words), Vec::<Finding>::new());
}

#[test]
fn lint_propagates_quoted_context_through_nested_operators() {
    let words = lex(r#""${X:-${Y:-$*}}""#).unwrap();

    assert_eq!(
        lint(&words),
        vec![finding(
            12,
            "W3",
            r#""$*" joins the parameters into one word; "$@" keeps them apart"#,
        )],
    );
}

#[test]
fn lint_respects_local_quotation_inside_an_unquoted_operator_word() {
    let words = lex(r#"${X:-"$Y"}"#).unwrap();

    assert_eq!(
        lint(&words),
        vec![finding(
            1,
            "W1",
            "unquoted parameter X is subject to splitting and pattern matching",
        )],
    );
}

#[test]
fn lint_checks_words_of_default_assign_alt_error_prefix_and_suffix_operators() {
    let words = lex(r#"${X:=$Y} ${X:+$Y} ${X:?$Y} ${X#$Y} ${X%$Y}"#).unwrap();

    assert_eq!(
        lint(&words),
        vec![
            finding(
                1,
                "W1",
                "unquoted parameter X is subject to splitting and pattern matching",
            ),
            finding(
                6,
                "W1",
                "unquoted parameter Y is subject to splitting and pattern matching",
            ),
            finding(
                10,
                "W1",
                "unquoted parameter X is subject to splitting and pattern matching",
            ),
            finding(
                15,
                "W1",
                "unquoted parameter Y is subject to splitting and pattern matching",
            ),
            finding(
                19,
                "W1",
                "unquoted parameter X is subject to splitting and pattern matching",
            ),
            finding(
                24,
                "W1",
                "unquoted parameter Y is subject to splitting and pattern matching",
            ),
            finding(
                28,
                "W1",
                "unquoted parameter X is subject to splitting and pattern matching",
            ),
            finding(
                32,
                "W1",
                "unquoted parameter Y is subject to splitting and pattern matching",
            ),
            finding(
                36,
                "W1",
                "unquoted parameter X is subject to splitting and pattern matching",
            ),
            finding(
                40,
                "W1",
                "unquoted parameter Y is subject to splitting and pattern matching",
            ),
        ],
    );
}

#[test]
fn lint_inherits_quote_context_through_explicit_ast_descendants() {
    let words = vec![Word {
        atoms: vec![parameter(
            Kind::Default,
            "X",
            1,
            true,
            vec![parameter(
                Kind::Default,
                "Y",
                6,
                false,
                vec![parameter(Kind::Plain, "*", 11, false, Vec::new())],
            )],
        )],
    }];
    let original = words.clone();

    assert_eq!(
        lint(&words),
        vec![finding(
            11,
            "W3",
            r#""$*" joins the parameters into one word; "$@" keeps them apart"#,
        )],
    );
    assert_eq!(words, original);
}

#[test]
fn lint_counts_unicode_and_line_feed_columns_as_characters() {
    let words = lex("é $X\n$Y").unwrap();

    assert_eq!(
        lint(&words),
        vec![
            finding(
                3,
                "W1",
                "unquoted parameter X is subject to splitting and pattern matching",
            ),
            finding(
                6,
                "W1",
                "unquoted parameter Y is subject to splitting and pattern matching",
            ),
        ],
    );
}

#[test]
fn lint_sorts_by_column_then_code_and_preserves_duplicate_findings() {
    let words = vec![Word {
        atoms: vec![
            parameter(Kind::Plain, "X", 9, false, Vec::new()),
            parameter(Kind::Plain, "@", 2, false, Vec::new()),
            parameter(Kind::Plain, "Y", 2, false, Vec::new()),
            parameter(Kind::Plain, "Y", 2, false, Vec::new()),
        ],
    }];

    assert_eq!(
        lint(&words),
        vec![
            finding(
                2,
                "W1",
                "unquoted parameter Y is subject to splitting and pattern matching",
            ),
            finding(
                2,
                "W1",
                "unquoted parameter Y is subject to splitting and pattern matching",
            ),
            finding(
                2,
                "W2",
                r#"unquoted $@ splits and matches its parameters again; write "$@""#,
            ),
            finding(
                9,
                "W1",
                "unquoted parameter X is subject to splitting and pattern matching",
            ),
        ],
    );
}

#[test]
fn lint_quote_markers_do_not_change_sibling_parameter_context() {
    let words = vec![Word {
        atoms: vec![parameter(
            Kind::Default,
            "X",
            1,
            false,
            vec![
                Atom::Quote,
                Atom::Ch {
                    ch: 'q',
                    quoted: true,
                },
                parameter(Kind::Plain, "Y", 6, false, Vec::new()),
            ],
        )],
    }];

    assert_eq!(
        lint(&words),
        vec![
            finding(
                1,
                "W1",
                "unquoted parameter X is subject to splitting and pattern matching",
            ),
            finding(
                6,
                "W1",
                "unquoted parameter Y is subject to splitting and pattern matching",
            ),
        ],
    );
}

#[test]
fn lint_inspects_operator_words_even_on_excluded_parent_parameters() {
    let words = vec![Word {
        atoms: vec![parameter(
            Kind::Length,
            "X",
            1,
            false,
            vec![parameter(Kind::Plain, "Y", 7, false, Vec::new())],
        )],
    }];

    assert_eq!(
        lint(&words),
        vec![finding(
            7,
            "W1",
            "unquoted parameter Y is subject to splitting and pattern matching",
        )],
    );
}
