use shexpand::atom::{Atom, Kind, Param, Word};
use shexpand::lexer::lex;
use shexpand::render::{atom_text, parts};

fn chars(text: &str, quoted: bool) -> Vec<Atom> {
    text.chars().map(|ch| Atom::Ch { ch, quoted }).collect()
}

fn plain(text: &str) -> Vec<Atom> {
    chars(text, false)
}

fn word(atoms: Vec<Atom>) -> Word {
    Word { atoms }
}

fn parameter(name: &str, text: &str, col: usize, quoted: bool) -> Atom {
    Atom::Param(Box::new(Param {
        kind: Kind::Plain,
        name: name.to_owned(),
        colon: false,
        longest: false,
        word: Vec::new(),
        text: text.to_owned(),
        col,
        quoted,
    }))
}

fn operated_parameter(
    kind: Kind,
    name: &str,
    colon: bool,
    longest: bool,
    operator_word: Vec<Atom>,
    text: &str,
    col: usize,
    quoted: bool,
) -> Atom {
    Atom::Param(Box::new(Param {
        kind,
        name: name.to_owned(),
        colon,
        longest,
        word: operator_word,
        text: text.to_owned(),
        col,
        quoted,
    }))
}

#[test]
fn lex_splits_on_space_tab_carriage_return_and_line_feed() {
    assert_eq!(
        lex(" a\tb\rc\nd ").unwrap(),
        vec![
            word(plain("a")),
            word(plain("b")),
            word(plain("c")),
            word(plain("d"))
        ]
    );
}

#[test]
fn lex_skips_comments_only_when_hash_starts_a_word() {
    assert_eq!(
        lex("# skip\na#b # tail\nc").unwrap(),
        vec![word(plain("a#b")), word(plain("c"))]
    );
}

#[test]
fn lex_keeps_brace_and_glob_punctuation_as_plain_characters() {
    assert_eq!(
        lex("{ } , ~ * ? [ ] ! = # tail").unwrap(),
        vec![
            word(plain("{")),
            word(plain("}")),
            word(plain(",")),
            word(plain("~")),
            word(plain("*")),
            word(plain("?")),
            word(plain("[")),
            word(plain("]")),
            word(plain("!")),
            word(plain("="))
        ]
    );
}

#[test]
fn lex_returns_no_words_for_empty_input() {
    assert_eq!(lex("").unwrap(), Vec::<Word>::new());
}

#[test]
fn lex_preserves_empty_single_and_double_quote_runs() {
    assert_eq!(
        lex("'' \"\"").unwrap(),
        vec![word(vec![Atom::Quote]), word(vec![Atom::Quote])]
    );
}

#[test]
fn lex_preserves_adjacent_single_and_double_quote_runs() {
    assert_eq!(
        lex("'a'\"b\"").unwrap(),
        vec![word(vec![
            Atom::Quote,
            Atom::Ch {
                ch: 'a',
                quoted: true
            },
            Atom::Quote,
            Atom::Ch {
                ch: 'b',
                quoted: true
            }
        ])]
    );
}

#[test]
fn lex_protects_line_feed_inside_single_quotes() {
    assert_eq!(
        lex("'a\nb'").unwrap(),
        vec![word(vec![
            Atom::Quote,
            Atom::Ch {
                ch: 'a',
                quoted: true
            },
            Atom::Ch {
                ch: '\n',
                quoted: true
            },
            Atom::Ch {
                ch: 'b',
                quoted: true
            }
        ])]
    );
}

#[test]
fn lex_quotes_the_character_after_an_outside_backslash() {
    assert_eq!(
        lex(r"a\ b").unwrap(),
        vec![word(vec![
            Atom::Ch {
                ch: 'a',
                quoted: false
            },
            Atom::Ch {
                ch: ' ',
                quoted: true
            },
            Atom::Ch {
                ch: 'b',
                quoted: false
            }
        ])]
    );
}

#[test]
fn lex_removes_outside_backslash_line_continuation() {
    assert_eq!(lex("a\\\nb").unwrap(), vec![word(plain("ab"))]);
}

#[test]
fn lex_does_not_create_a_word_for_continuation_alone() {
    assert_eq!(lex("\\\n").unwrap(), Vec::<Word>::new());
}

#[test]
fn lex_handles_double_quote_escape_characters_and_retained_backslashes() {
    assert_eq!(
        lex(r#""\$\`\"\\\q""#).unwrap(),
        vec![word(vec![
            Atom::Quote,
            Atom::Ch {
                ch: '$',
                quoted: true
            },
            Atom::Ch {
                ch: '`',
                quoted: true
            },
            Atom::Ch {
                ch: '"',
                quoted: true
            },
            Atom::Ch {
                ch: '\\',
                quoted: true
            },
            Atom::Ch {
                ch: '\\',
                quoted: true
            },
            Atom::Ch {
                ch: 'q',
                quoted: true
            }
        ])]
    );
}

#[test]
fn lex_removes_line_continuation_inside_double_quotes() {
    assert_eq!(
        lex("\"a\\\nb\"").unwrap(),
        vec![word(vec![
            Atom::Quote,
            Atom::Ch {
                ch: 'a',
                quoted: true
            },
            Atom::Ch {
                ch: 'b',
                quoted: true
            }
        ])]
    );
}

#[test]
fn lex_leaves_marker_free_double_quoted_at_without_parameters() {
    assert_eq!(
        lex("\"$@\"").unwrap(),
        vec![word(vec![parameter("@", "$@", 2, true)])]
    );
}

#[test]
fn lex_leaves_marker_free_repeated_double_quoted_at_parameters() {
    assert_eq!(
        lex("\"$@${@}\"").unwrap(),
        vec![word(vec![
            parameter("@", "$@", 2, true),
            parameter("@", "${@}", 4, true)
        ])]
    );
}

#[test]
fn lex_keeps_a_quote_marker_when_double_quoted_at_has_literal_text() {
    assert_eq!(
        lex("\"$@x\"").unwrap(),
        vec![word(vec![
            Atom::Quote,
            parameter("@", "$@", 2, true),
            Atom::Ch {
                ch: 'x',
                quoted: true
            }
        ])]
    );
}

#[test]
fn lex_keeps_a_quote_marker_for_double_quoted_star() {
    assert_eq!(
        lex("\"$*\"").unwrap(),
        vec![word(vec![Atom::Quote, parameter("*", "$*", 2, true)])]
    );
}

#[test]
fn lex_retains_empty_quote_marker_before_marker_free_at_run() {
    assert_eq!(
        lex("\"\"\"$@\"").unwrap(),
        vec![word(vec![Atom::Quote, parameter("@", "$@", 4, true)])]
    );
}

#[test]
fn lex_parses_unbraced_names_numbers_and_special_parameters() {
    assert_eq!(
        lex("$ABC_2 $_x $1 $9 $10 $@ $* $#").unwrap(),
        vec![
            word(vec![parameter("ABC_2", "$ABC_2", 1, false)]),
            word(vec![parameter("_x", "$_x", 8, false)]),
            word(vec![parameter("1", "$1", 12, false)]),
            word(vec![parameter("9", "$9", 15, false)]),
            word(vec![
                parameter("1", "$1", 18, false),
                Atom::Ch {
                    ch: '0',
                    quoted: false
                }
            ]),
            word(vec![parameter("@", "$@", 22, false)]),
            word(vec![parameter("*", "$*", 25, false)]),
            word(vec![parameter("#", "$#", 28, false)])
        ]
    );
}

#[test]
fn lex_preserves_literal_dollars_before_unrecognized_characters() {
    assert_eq!(
        lex("$ $: $é").unwrap(),
        vec![word(plain("$")), word(plain("$:")), word(plain("$é"))]
    );
}

#[test]
fn lex_treats_dollar_single_quote_as_literal_inside_double_quotes() {
    assert_eq!(
        lex("\"$'\"").unwrap(),
        vec![word(vec![
            Atom::Quote,
            Atom::Ch {
                ch: '$',
                quoted: true
            },
            Atom::Ch {
                ch: '\'',
                quoted: true
            }
        ])]
    );
}

#[test]
fn lex_treats_dollar_before_closing_double_quote_as_literal() {
    assert_eq!(
        lex("\"$\"").unwrap(),
        vec![word(vec![
            Atom::Quote,
            Atom::Ch {
                ch: '$',
                quoted: true
            }
        ])]
    );
}

#[test]
fn lex_parses_all_plain_braced_parameter_forms() {
    assert_eq!(
        lex("${#} ${X} ${1} ${99} ${@} ${*}").unwrap(),
        vec![
            word(vec![operated_parameter(
                Kind::Plain,
                "#",
                false,
                false,
                vec![],
                "${#}",
                1,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Plain,
                "X",
                false,
                false,
                vec![],
                "${X}",
                6,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Plain,
                "1",
                false,
                false,
                vec![],
                "${1}",
                11,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Plain,
                "99",
                false,
                false,
                vec![],
                "${99}",
                16,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Plain,
                "@",
                false,
                false,
                vec![],
                "${@}",
                22,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Plain,
                "*",
                false,
                false,
                vec![],
                "${*}",
                27,
                false
            )])
        ]
    );
}

#[test]
fn lex_parses_length_parameter_forms() {
    assert_eq!(
        lex("${#X} ${#99}").unwrap(),
        vec![
            word(vec![operated_parameter(
                Kind::Length,
                "X",
                false,
                false,
                vec![],
                "${#X}",
                1,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Length,
                "99",
                false,
                false,
                vec![],
                "${#99}",
                7,
                false
            )])
        ]
    );
}

#[test]
fn lex_parses_default_assign_alt_and_error_operators() {
    assert_eq!(
        lex("${X-a} ${X:-a} ${X=a} ${X:=a} ${X+a} ${X:+a} ${X?a} ${X:?a}").unwrap(),
        vec![
            word(vec![operated_parameter(
                Kind::Default,
                "X",
                false,
                false,
                plain("a"),
                "${X-a}",
                1,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Default,
                "X",
                true,
                false,
                plain("a"),
                "${X:-a}",
                8,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Assign,
                "X",
                false,
                false,
                plain("a"),
                "${X=a}",
                16,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Assign,
                "X",
                true,
                false,
                plain("a"),
                "${X:=a}",
                23,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Alt,
                "X",
                false,
                false,
                plain("a"),
                "${X+a}",
                31,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Alt,
                "X",
                true,
                false,
                plain("a"),
                "${X:+a}",
                38,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Error,
                "X",
                false,
                false,
                plain("a"),
                "${X?a}",
                46,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Error,
                "X",
                true,
                false,
                plain("a"),
                "${X:?a}",
                53,
                false
            )])
        ]
    );
}

#[test]
fn lex_parses_shortest_and_longest_prefix_and_suffix_removal() {
    assert_eq!(
        lex("${X#a} ${X##a} ${X%a} ${X%%a}").unwrap(),
        vec![
            word(vec![operated_parameter(
                Kind::Prefix,
                "X",
                false,
                false,
                plain("a"),
                "${X#a}",
                1,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Prefix,
                "X",
                false,
                true,
                plain("a"),
                "${X##a}",
                8,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Suffix,
                "X",
                false,
                false,
                plain("a"),
                "${X%a}",
                16,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Suffix,
                "X",
                false,
                true,
                plain("a"),
                "${X%%a}",
                23,
                false
            )])
        ]
    );
}

#[test]
fn lex_keeps_empty_operator_words_and_default_flags() {
    assert_eq!(
        lex("${X:-} ${X%}").unwrap(),
        vec![
            word(vec![operated_parameter(
                Kind::Default,
                "X",
                true,
                false,
                vec![],
                "${X:-}",
                1,
                false
            )]),
            word(vec![operated_parameter(
                Kind::Suffix,
                "X",
                false,
                false,
                vec![],
                "${X%}",
                8,
                false
            )])
        ]
    );
}

#[test]
fn lex_keeps_blanks_operators_and_hash_plain_in_operator_words() {
    assert_eq!(
        lex("${X:-a b|&;<>()#}").unwrap(),
        vec![word(vec![operated_parameter(
            Kind::Default,
            "X",
            true,
            false,
            plain("a b|&;<>()#"),
            "${X:-a b|&;<>()#}",
            1,
            false
        )])]
    );
}

#[test]
fn lex_protects_operator_word_closing_braces_with_quotes_and_escape() {
    assert_eq!(
        lex(r#"${X:-'a}'"b}"\}}"#).unwrap(),
        vec![word(vec![operated_parameter(
            Kind::Default,
            "X",
            true,
            false,
            vec![
                Atom::Quote,
                Atom::Ch {
                    ch: 'a',
                    quoted: true
                },
                Atom::Ch {
                    ch: '}',
                    quoted: true
                },
                Atom::Quote,
                Atom::Ch {
                    ch: 'b',
                    quoted: true
                },
                Atom::Ch {
                    ch: '}',
                    quoted: true
                },
                Atom::Ch {
                    ch: '}',
                    quoted: true
                }
            ],
            r#"${X:-'a}'"b}"\}}"#,
            1,
            false
        )])]
    );
}

#[test]
fn lex_parses_nested_operator_parameters_recursively() {
    assert_eq!(
        lex("${X:-${Y:=z}}").unwrap(),
        vec![word(vec![operated_parameter(
            Kind::Default,
            "X",
            true,
            false,
            vec![operated_parameter(
                Kind::Assign,
                "Y",
                true,
                false,
                plain("z"),
                "${Y:=z}",
                6,
                false
            )],
            "${X:-${Y:=z}}",
            1,
            false
        )])]
    );
}

#[test]
fn lex_propagates_double_quote_context_to_nested_parameters() {
    assert_eq!(
        lex("\"${X:-$Y}\"").unwrap(),
        vec![word(vec![
            Atom::Quote,
            operated_parameter(
                Kind::Default,
                "X",
                true,
                false,
                vec![parameter("Y", "$Y", 7, true)],
                "${X:-$Y}",
                2,
                true
            )
        ])]
    );
}

#[test]
fn lex_keeps_operator_word_characters_plain_in_double_quote_context() {
    assert_eq!(
        lex(r#""${X:-a 'b' \q}""#).unwrap(),
        vec![word(vec![
            Atom::Quote,
            operated_parameter(
                Kind::Default,
                "X",
                true,
                false,
                plain(r"a 'b' \q"),
                r"${X:-a 'b' \q}",
                2,
                true
            )
        ])]
    );
}

#[test]
fn lex_handles_special_escapes_in_double_context_operator_word() {
    assert_eq!(
        lex(r#""${X:-\$\`\\\}z}""#).unwrap(),
        vec![word(vec![
            Atom::Quote,
            operated_parameter(
                Kind::Default,
                "X",
                true,
                false,
                vec![
                    Atom::Ch {
                        ch: '$',
                        quoted: true
                    },
                    Atom::Ch {
                        ch: '`',
                        quoted: true
                    },
                    Atom::Ch {
                        ch: '\\',
                        quoted: true
                    },
                    Atom::Ch {
                        ch: '}',
                        quoted: true
                    },
                    Atom::Ch {
                        ch: 'z',
                        quoted: false
                    }
                ],
                r#"${X:-\$\`\\\}z}"#,
                2,
                true
            )
        ])]
    );
}

#[test]
fn lex_removes_line_continuation_in_double_context_operator_word() {
    assert_eq!(
        lex("\"${X:-a\\\nb}\"").unwrap(),
        vec![word(vec![
            Atom::Quote,
            operated_parameter(
                Kind::Default,
                "X",
                true,
                false,
                plain("ab"),
                "${X:-a\\\nb}",
                2,
                true
            )
        ])]
    );
}

#[test]
fn lex_accepts_escaped_double_quote_in_double_context_operator_word() {
    assert_eq!(
        lex(r#""${X:-\"}""#).unwrap(),
        vec![word(vec![
            Atom::Quote,
            operated_parameter(
                Kind::Default,
                "X",
                true,
                false,
                chars("\"", true),
                r#"${X:-\"}"#,
                2,
                true
            )
        ])]
    );
}

#[test]
fn lex_reports_unterminated_quotes_and_trailing_backslash_at_their_columns() {
    assert_eq!(
        lex("echo 'a").unwrap_err().0,
        "unterminated single quote starting at column 6"
    );
    assert_eq!(
        lex("echo \"a").unwrap_err().0,
        "unterminated double quote starting at column 6"
    );
    assert_eq!(
        lex(r"echo a\").unwrap_err().0,
        "trailing backslash at column 7"
    );
}

#[test]
fn lex_reports_every_unsupported_operator_with_the_character_and_column() {
    assert_eq!(
        lex("a | b").unwrap_err().0,
        "unsupported operator \"|\" at column 3"
    );
    assert_eq!(
        lex("&").unwrap_err().0,
        "unsupported operator \"&\" at column 1"
    );
    assert_eq!(
        lex(";").unwrap_err().0,
        "unsupported operator \";\" at column 1"
    );
    assert_eq!(
        lex("<").unwrap_err().0,
        "unsupported operator \"<\" at column 1"
    );
    assert_eq!(
        lex(">").unwrap_err().0,
        "unsupported operator \">\" at column 1"
    );
    assert_eq!(
        lex("(").unwrap_err().0,
        "unsupported operator \"(\" at column 1"
    );
    assert_eq!(
        lex(")").unwrap_err().0,
        "unsupported operator \")\" at column 1"
    );
}

#[test]
fn lex_rejects_backquote_and_dollar_parenthesis_command_substitution() {
    assert_eq!(
        lex(r"echo `ls`").unwrap_err().0,
        "command substitution is not supported at column 6"
    );
    assert_eq!(
        lex("echo $(ls)").unwrap_err().0,
        "command substitution is not supported at column 6"
    );
}

#[test]
fn lex_rejects_backquotes_in_double_quotes_and_operator_words() {
    assert_eq!(
        lex(r#""`""#).unwrap_err().0,
        "command substitution is not supported at column 2"
    );
    assert_eq!(
        lex(r"${X:-`}").unwrap_err().0,
        "command substitution is not supported at column 6"
    );
}

#[test]
fn lex_accepts_backquote_inside_a_single_quoted_run() {
    assert_eq!(
        lex(r"'`'").unwrap(),
        vec![word(vec![
            Atom::Quote,
            Atom::Ch {
                ch: '`',
                quoted: true
            }
        ])]
    );
}

#[test]
fn lex_reports_unsupported_dollar_parameter_forms() {
    assert_eq!(
        lex("$0").unwrap_err().0,
        "unsupported parameter \"$0\" at column 1"
    );
    assert_eq!(
        lex("$?").unwrap_err().0,
        "unsupported parameter \"$?\" at column 1"
    );
    assert_eq!(
        lex("$$").unwrap_err().0,
        "unsupported parameter \"$$\" at column 1"
    );
    assert_eq!(
        lex("$!").unwrap_err().0,
        "unsupported parameter \"$!\" at column 1"
    );
    assert_eq!(
        lex("$-").unwrap_err().0,
        "unsupported parameter \"$-\" at column 1"
    );
}

#[test]
fn lex_rejects_dollar_quote_forms_outside_double_quotes() {
    assert_eq!(
        lex("echo $\"a\"").unwrap_err().0,
        "unsupported quoting after $ at column 6"
    );
    assert_eq!(
        lex("echo $'a'").unwrap_err().0,
        "unsupported quoting after $ at column 6"
    );
}

#[test]
fn lex_reports_bad_substitutions_for_invalid_braced_parameters() {
    assert_eq!(
        lex("${}").unwrap_err().0,
        "bad substitution \"${}\" at column 1"
    );
    assert_eq!(
        lex("${0}").unwrap_err().0,
        "bad substitution \"${0}\" at column 1"
    );
    assert_eq!(
        lex("${01}").unwrap_err().0,
        "bad substitution \"${01}\" at column 1"
    );
    assert_eq!(
        lex("${100}").unwrap_err().0,
        "bad substitution \"${100}\" at column 1"
    );
    assert_eq!(
        lex("${@:-x}").unwrap_err().0,
        "bad substitution \"${@:-x}\" at column 1"
    );
    assert_eq!(
        lex("${*%x}").unwrap_err().0,
        "bad substitution \"${*%x}\" at column 1"
    );
    assert_eq!(
        lex("${#@}").unwrap_err().0,
        "bad substitution \"${#@}\" at column 1"
    );
    assert_eq!(
        lex("${#*}").unwrap_err().0,
        "bad substitution \"${#*}\" at column 1"
    );
    assert_eq!(
        lex("${X:}").unwrap_err().0,
        "bad substitution \"${X:}\" at column 1"
    );
    assert_eq!(
        lex("echo ${X^}").unwrap_err().0,
        "bad substitution \"${X^}\" at column 6"
    );
}

#[test]
fn lex_sanitizes_unicode_in_bad_substitution_diagnostics() {
    assert_eq!(
        lex("${é}").unwrap_err().0,
        "bad substitution \"${?}\" at column 1"
    );
}

#[test]
fn lex_limits_bad_substitution_text_to_first_closing_brace_or_end() {
    assert_eq!(
        lex("${X^abc").unwrap_err().0,
        "bad substitution \"${X^abc\" at column 1"
    );
    assert_eq!(
        lex("${X^${Y}}").unwrap_err().0,
        "bad substitution \"${X^${Y}\" at column 1"
    );
}

#[test]
fn lex_reports_unterminated_valid_prefixes_and_inner_double_quote() {
    assert_eq!(
        lex("echo ${A:-b").unwrap_err().0,
        "unterminated ${ starting at column 6"
    );
    assert_eq!(
        lex("echo ${A").unwrap_err().0,
        "unterminated ${ starting at column 6"
    );
    assert_eq!(
        lex("echo \"${X:-\"a\"}\"").unwrap_err().0,
        "unsupported double quote inside \"${...}\" at column 12"
    );
}

fn nested_parameter_depth(atom: &Atom) -> usize {
    match atom {
        Atom::Param(param) => {
            1 + param
                .word
                .iter()
                .map(nested_parameter_depth)
                .max()
                .unwrap_or(0)
        }
        Atom::Ch { .. } | Atom::Quote => 0,
    }
}

fn deepest_parameter(atom: &Atom) -> &Param {
    let Atom::Param(param) = atom else {
        panic!("expected a parameter atom")
    };

    param
        .word
        .iter()
        .find(|child| matches!(child, Atom::Param(_)))
        .map(deepest_parameter)
        .unwrap_or(param)
}

#[test]
fn lex_counts_unicode_columns_by_characters() {
    let words = lex("é $X").unwrap();
    let Atom::Param(param) = &words[1].atoms[0] else {
        panic!("expected a parameter atom")
    };
    assert_eq!(param.col, 3);
    assert_eq!(param.text, "$X");
}

#[test]
fn lex_keeps_columns_global_across_line_feeds() {
    let words = lex("é\n$X").unwrap();
    let Atom::Param(param) = &words[1].atoms[0] else {
        panic!("expected a parameter atom")
    };
    assert_eq!(param.col, 3);
    assert_eq!(param.text, "$X");
}

#[test]
fn lex_counts_quoted_unicode_columns_from_the_original_source() {
    let words = lex("\"é$X\"").unwrap();
    let Atom::Param(param) = &words[0].atoms[2] else {
        panic!("expected a parameter atom")
    };
    assert_eq!(param.col, 3);
    assert!(param.quoted);
}

#[test]
fn lex_accepts_eight_nested_braced_substitutions() {
    let words = lex("${A:-${A:-${A:-${A:-${A:-${A:-${A:-${A:-x}}}}}}}}}").unwrap();
    assert_eq!(words.len(), 1);
    assert_eq!(nested_parameter_depth(&words[0].atoms[0]), 8);
    assert_eq!(deepest_parameter(&words[0].atoms[0]).word, plain("x"));
}

#[test]
fn lex_rejects_the_ninth_nested_braced_substitution_at_its_column() {
    assert_eq!(
        lex("${A:-${A:-${A:-${A:-${A:-${A:-${A:-${A:-${A:-x}}}}}}}}}")
            .unwrap_err()
            .0,
        "nesting is deeper than 8 at column 41"
    );
}

#[test]
fn parts_merges_adjacent_quoted_runs_and_atom_text_omits_markers() {
    let words = lex("'a'\"b\"").unwrap();
    assert_eq!(parts(&words[0].atoms), vec!["q[ab]"]);
    assert_eq!(atom_text(&words[0].atoms), "ab");
}

#[test]
fn parts_keep_plain_and_quoted_runs_separate_from_parameters() {
    let words = lex("a'$'\"$X\"z").unwrap();
    assert_eq!(
        parts(&words[0].atoms),
        vec!["u[a]", "q[$]", "P[$X]", "u[z]"]
    );
    assert_eq!(atom_text(&words[0].atoms), "a$$Xz");
}

#[test]
fn parts_preserve_an_empty_quote_run() {
    let words = lex("''").unwrap();
    assert_eq!(parts(&words[0].atoms), vec!["q[]"]);
    assert_eq!(atom_text(&words[0].atoms), "");
}

#[test]
fn parts_render_parameter_source_opaquely() {
    let words = lex("${X:-'a b'}").unwrap();
    assert_eq!(parts(&words[0].atoms), vec!["p[${X:-'a b'}]"]);
    assert_eq!(atom_text(&words[0].atoms), "${X:-'a b'}");
}

#[test]
fn parts_sanitize_non_ascii_and_newline_but_atom_text_stays_raw() {
    let words = lex("'é\n[]'").unwrap();
    assert_eq!(parts(&words[0].atoms), vec!["q[??[]]"]);
    assert_eq!(atom_text(&words[0].atoms), "é\n[]");
}

#[test]
fn rendering_empty_atom_slices_returns_empty_text_and_no_parts() {
    assert_eq!(parts(&[]), Vec::<String>::new());
    assert_eq!(atom_text(&[]), "");
}

#[test]
fn atom_text_uses_stored_parameter_source_instead_of_reconstructing_it() {
    let atoms = vec![parameter("different", "$RAW", 1, false)];
    assert_eq!(atom_text(&atoms), "$RAW");
    assert_eq!(parts(&atoms), vec!["p[$RAW]"]);
}

#[test]
fn lex_accepts_braced_parameter_number_thirty() {
    assert_eq!(
        lex("${30}").unwrap(),
        vec![word(vec![operated_parameter(
            Kind::Plain,
            "30",
            false,
            false,
            vec![],
            "${30}",
            1,
            false
        )])]
    );
}
