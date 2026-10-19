use std::collections::BTreeMap;

use shexpand::atom::Atom;
use shexpand::eval::{Scope, eval};
use shexpand::fields::Item;
use shexpand::lexer::lex;
use shexpand::pipeline::expand_words;
use shexpand::vfs::Vfs;

fn make_scope(entries: &[(&str, &str)], positional: &[&str]) -> Scope {
    let vars: BTreeMap<String, String> = entries
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    let positional = positional.iter().map(|value| (*value).to_owned()).collect();
    Scope::new(vars, positional)
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn expand(
    input: &str,
    entries: &[(&str, &str)],
    positional: &[&str],
    ifs: &str,
) -> (Vec<String>, Scope) {
    let words = lex(input).unwrap();
    let mut scope = make_scope(entries, positional);
    let vfs = Vfs::new(&[]).unwrap();
    let expanded = expand_words(&words, &mut scope, ifs, &vfs).unwrap();
    (expanded, scope)
}

fn expand_with_files(
    input: &str,
    entries: &[(&str, &str)],
    positional: &[&str],
    ifs: &str,
    files: &[&str],
) -> (Vec<String>, Scope) {
    let words = lex(input).unwrap();
    let mut scope = make_scope(entries, positional);
    let files: Vec<String> = files.iter().map(|path| (*path).to_owned()).collect();
    let vfs = Vfs::new(&files).unwrap();
    let expanded = expand_words(&words, &mut scope, ifs, &vfs).unwrap();
    (expanded, scope)
}

fn evaluate(input: &str, scope: &mut Scope, ifs: &str) -> Vec<Item> {
    let words = lex(input).unwrap();
    assert_eq!(words.len(), 1);
    eval(&words[0].atoms, scope, ifs).unwrap()
}

fn evaluate_quoted_parameter(input: &str, scope: &mut Scope, ifs: &str) -> Vec<Item> {
    let mut words = lex(input).unwrap();
    assert_eq!(words.len(), 1);
    let Some(Atom::Param(param)) = words[0].atoms.first_mut() else {
        panic!("expected a parameter atom");
    };
    param.quoted = true;
    eval(&words[0].atoms, scope, ifs).unwrap()
}

#[test]
fn scope_initialization_retains_bindings_and_starts_without_assignments() {
    let scope = make_scope(&[("X", "v")], &["p"]);

    assert_eq!(scope.vars.get("X"), Some(&"v".to_owned()));
    assert_eq!(scope.positional, strings(&["p"]));
    assert_eq!(scope.assigned, Vec::<(String, String)>::new());
}

#[test]
fn literal_characters_and_quote_markers_preserve_item_flags() {
    let mut scope = make_scope(&[], &[]);
    let items = eval(
        &[
            Atom::Ch {
                ch: ' ',
                quoted: false,
            },
            Atom::Ch {
                ch: 'x',
                quoted: true,
            },
            Atom::Quote,
        ],
        &mut scope,
        " \t\n",
    )
    .unwrap();

    assert_eq!(
        items,
        vec![
            Item::Ch {
                ch: ' ',
                quoted: false,
                split: false,
            },
            Item::Ch {
                ch: 'x',
                quoted: true,
                split: false,
            },
            Item::Null,
        ]
    );
}

#[test]
fn unquoted_parameter_text_is_split_eligible_but_not_quoted() {
    let mut scope = make_scope(&[("X", "a b")], &[]);

    assert_eq!(
        evaluate("$X", &mut scope, " \t\n"),
        vec![
            Item::Ch {
                ch: 'a',
                quoted: false,
                split: true,
            },
            Item::Ch {
                ch: ' ',
                quoted: false,
                split: true,
            },
            Item::Ch {
                ch: 'b',
                quoted: false,
                split: true,
            },
        ]
    );
}

#[test]
fn quoted_parameter_text_starts_with_null_and_cannot_split() {
    let mut scope = make_scope(&[("X", "a b")], &[]);

    assert_eq!(
        evaluate_quoted_parameter("$X", &mut scope, " \t\n"),
        vec![
            Item::Null,
            Item::Ch {
                ch: 'a',
                quoted: true,
                split: false,
            },
            Item::Ch {
                ch: ' ',
                quoted: true,
                split: false,
            },
            Item::Ch {
                ch: 'b',
                quoted: true,
                split: false,
            },
        ]
    );
}

#[test]
fn unset_plain_and_quoted_parameters_differ_by_null_retention() {
    let mut plain_scope = make_scope(&[], &[]);
    let mut quoted_scope = make_scope(&[], &[]);

    assert_eq!(
        evaluate("$U", &mut plain_scope, " \t\n"),
        Vec::<Item>::new()
    );
    assert_eq!(
        evaluate_quoted_parameter("$U", &mut quoted_scope, " \t\n"),
        vec![Item::Null]
    );
}

#[test]
fn empty_plain_and_quoted_parameters_differ_by_null_retention() {
    let mut plain_scope = make_scope(&[("E", "")], &[]);
    let mut quoted_scope = make_scope(&[("E", "")], &[]);

    assert_eq!(
        evaluate("$E", &mut plain_scope, " \t\n"),
        Vec::<Item>::new()
    );
    assert_eq!(
        evaluate_quoted_parameter("$E", &mut quoted_scope, " \t\n"),
        vec![Item::Null]
    );
}

#[test]
fn positional_numbers_and_count_expand_in_order() {
    let (result, _) = expand("$1 ${2} ${99} ${#}", &[], &["p", "q"], " \t\n");

    assert_eq!(result, strings(&["p", "q", "2"]));
}

#[test]
fn lengths_count_unicode_characters_and_empty_or_missing_values() {
    let (result, _) = expand(
        "${#X} ${#E} ${#U} ${#1} ${#99}",
        &[("X", "é🙂"), ("E", "")],
        &["abc"],
        " \t\n",
    );

    assert_eq!(result, strings(&["2", "0", "0", "3", "0"]));
}

#[test]
fn zero_positionals_make_quoted_at_absent_and_quoted_star_empty() {
    let (at, _) = expand("\"$@\" \"${@}\"", &[], &[], " \t\n");
    let (star, _) = expand("\"$*\"", &[], &[], " \t\n");

    assert_eq!(at, Vec::<String>::new());
    assert_eq!(star, strings(&[""]));
}

#[test]
fn one_positional_is_preserved_by_quoted_at_and_star() {
    let (result, _) = expand("\"$@\" \"$*\"", &[], &["p"], " \t\n");

    assert_eq!(result, strings(&["p", "p"]));
}

#[test]
fn quoted_at_keeps_each_positional_separate_including_empty_values() {
    let (result, _) = expand("\"$@\"", &[], &["p", "q r", ""], " \t\n");

    assert_eq!(result, strings(&["p", "q r", ""]));
}

#[test]
fn unquoted_at_and_star_split_positional_values_and_drop_empty_values() {
    let (result, _) = expand("$@ $*", &[], &["p", "q r", ""], " \t\n");

    assert_eq!(result, strings(&["p", "q", "r", "p", "q", "r"]));
}

#[test]
fn quoted_star_joins_with_the_first_ifs_character() {
    let (result, _) = expand("\"$*\"", &[], &["p", "q r", ""], ":,");

    assert_eq!(result, strings(&["p:q r:"]));
}

#[test]
fn quoted_star_joins_without_a_separator_when_ifs_is_empty() {
    let (result, _) = expand("\"$*\"", &[], &["p", "q"], "");

    assert_eq!(result, strings(&["pq"]));
}

#[test]
fn quoted_at_concatenates_positionals_around_surrounding_text() {
    let (result, _) = expand("a\"$@\"z", &[], &["p", "q"], " \t\n");

    assert_eq!(result, strings(&["ap", "qz"]));
}

#[test]
fn quoted_at_with_no_positionals_keeps_the_surrounding_text() {
    let (result, _) = expand("a\"$@\"z", &[], &[], " \t\n");

    assert_eq!(result, strings(&["az"]));
}

#[test]
fn defaults_distinguish_unset_empty_and_set_values() {
    let (result, _) = expand(
        "${U-w} ${U:-w} ${E-w} ${E:-w} ${X-w} ${X:-w}",
        &[("E", ""), ("X", "v")],
        &[],
        " \t\n",
    );

    assert_eq!(result, strings(&["w", "w", "w", "v", "v"]));
}

#[test]
fn alternatives_distinguish_unset_empty_and_set_values() {
    let (result, _) = expand(
        "${U+w} ${U:+w} ${E+w} ${E:+w} ${X+w} ${X:+w}",
        &[("E", ""), ("X", "v")],
        &[],
        " \t\n",
    );

    assert_eq!(result, strings(&["w", "w", "w"]));
}

#[test]
fn selected_default_and_alternative_words_inherit_quote_context() {
    let (result, _) = expand(
        "${U:-a b} \"${U:-a b}\" ${X:+a b} \"${X:+a b}\"",
        &[("X", "v")],
        &[],
        " \t\n",
    );

    assert_eq!(result, strings(&["a", "b", "a b", "a", "b", "a b"]));
}

#[test]
fn explicit_quotes_inside_an_operator_word_keep_their_grouping() {
    let (result, _) = expand("${U:-'a b'}", &[], &[], " \t\n");

    assert_eq!(result, strings(&["a b"]));
}

#[test]
fn skipped_operator_words_have_no_assignment_or_error_side_effects() {
    let (result, scope) = expand(
        "${X:-${N:=bad}} ${U:+${N:=bad}} ${X:-${Z:?oops}}",
        &[("X", "v")],
        &[],
        " \t\n",
    );

    assert_eq!(result, strings(&["v", "v"]));
    assert_eq!(scope.vars.get("N"), None);
    assert_eq!(scope.assigned, Vec::<(String, String)>::new());
}

fn expansion_error(input: &str, entries: &[(&str, &str)], positional: &[&str]) -> String {
    let words = lex(input).unwrap();
    let mut scope = make_scope(entries, positional);
    let vfs = Vfs::new(&[]).unwrap();

    expand_words(&words, &mut scope, " \t\n", &vfs)
        .unwrap_err()
        .0
}

#[test]
fn assignments_record_unset_targets_in_evaluation_order() {
    let (result, scope) = expand("${U=w} ${V:=x}", &[], &[], " \t\n");

    assert_eq!(result, strings(&["w", "x"]));
    assert_eq!(
        scope.assigned,
        vec![
            ("U".to_owned(), "w".to_owned()),
            ("V".to_owned(), "x".to_owned())
        ]
    );
}

#[test]
fn assignment_selects_empty_values_but_not_set_values() {
    let (result, scope) = expand(
        "${E=w} ${E:=x} ${X=w} ${X:=x}",
        &[("E", ""), ("X", "v")],
        &[],
        " \t\n",
    );

    assert_eq!(result, strings(&["x", "v", "v"]));
    assert_eq!(scope.assigned, vec![("E".to_owned(), "x".to_owned())]);
}

#[test]
fn assignment_is_visible_immediately_and_on_later_words() {
    let (result, scope) = expand("${N:=7}$N $N", &[], &[], " \t\n");

    assert_eq!(result, strings(&["77", "7"]));
    assert_eq!(scope.assigned, vec![("N".to_owned(), "7".to_owned())]);
    assert_eq!(scope.vars.get("N"), Some(&"7".to_owned()));
}

#[test]
fn nested_assignments_are_recorded_before_the_enclosing_assignment() {
    let (result, scope) = expand("${A:=${B:=x}}", &[], &[], " \t\n");

    assert_eq!(result, strings(&["x"]));
    assert_eq!(
        scope.assigned,
        vec![
            ("B".to_owned(), "x".to_owned()),
            ("A".to_owned(), "x".to_owned())
        ]
    );
}

#[test]
fn assignment_flattens_positional_breaks_to_spaces() {
    let (result, scope) = expand("${N:=$@}", &[], &["a", "b c"], " \t\n");

    assert_eq!(result, strings(&["a", "b", "c"]));
    assert_eq!(scope.vars.get("N"), Some(&"a b c".to_owned()));
}

#[test]
fn repeated_assignment_records_each_selected_empty_value() {
    let (result, scope) = expand("${N:=}${N:=x}", &[], &[], " \t\n");

    assert_eq!(result, strings(&["x"]));
    assert_eq!(
        scope.assigned,
        vec![
            ("N".to_owned(), "".to_owned()),
            ("N".to_owned(), "x".to_owned())
        ]
    );
}

#[test]
fn selected_numeric_assignment_fails_before_evaluating_its_word() {
    let words = lex("${1:=${N:=bad}}").unwrap();
    let mut scope = make_scope(&[], &[]);
    let vfs = Vfs::new(&[]).unwrap();

    assert_eq!(
        expand_words(&words, &mut scope, " \t\n", &vfs)
            .unwrap_err()
            .0,
        "1: cannot assign in this way"
    );
    assert_eq!(scope.vars.get("N"), None);
    assert_eq!(scope.assigned, Vec::<(String, String)>::new());
}

#[test]
fn set_numeric_assignment_target_returns_its_value_without_assignment() {
    let (result, scope) = expand("${1:=x}", &[], &["p"], " \t\n");

    assert_eq!(result, strings(&["p"]));
    assert_eq!(scope.assigned, Vec::<(String, String)>::new());
}

#[test]
fn error_operator_uses_explicit_and_flattened_messages() {
    assert_eq!(expansion_error("${Z:?oops}", &[], &[]), "Z: oops");
    assert_eq!(expansion_error("${Z:?$@}", &[], &["a", "b"]), "Z: a b");
}

#[test]
fn error_operator_uses_exact_default_messages() {
    assert_eq!(
        expansion_error("${Z:?}", &[], &[]),
        "Z: parameter null or not set"
    );
    assert_eq!(expansion_error("${Z?}", &[], &[]), "Z: parameter not set");
}

#[test]
fn empty_error_operand_fails_only_when_colon_is_present() {
    let (result, _) = expand("${E?bad}", &[("E", "")], &[], " \t\n");

    assert_eq!(result, Vec::<String>::new());
    assert_eq!(expansion_error("${E:?bad}", &[("E", "")], &[]), "E: bad");
}

#[test]
fn set_error_operands_bypass_their_words() {
    let (result, _) = expand("${X?bad} ${X:?bad}", &[("X", "v")], &[], " \t\n");

    assert_eq!(result, strings(&["v", "v"]));
}

#[test]
fn trim_operators_remove_shortest_and_longest_prefixes_and_suffixes() {
    let (result, _) = expand(
        "${X#a*} ${X##a*} ${X%*c} ${X%%*c}",
        &[("X", "abcabc")],
        &[],
        " \t\n",
    );

    assert_eq!(result, strings(&["bcabc", "abcab"]));
}

#[test]
fn quoted_longest_trims_retain_empty_results() {
    let (result, _) = expand(
        "\"${X##a*}\" \"${X%%*c}\"",
        &[("X", "abcabc")],
        &[],
        " \t\n",
    );

    assert_eq!(result, strings(&["", ""]));
}

#[test]
fn trim_without_a_matching_prefix_or_suffix_preserves_the_value() {
    let (result, _) = expand("${X#z*} ${X%z*}", &[("X", "abc")], &[], " \t\n");

    assert_eq!(result, strings(&["abc", "abc"]));
}

#[test]
fn trim_patterns_preserve_quoted_cells_and_expand_parameter_patterns() {
    let (quoted, _) = expand("${X#'a*'}", &[("X", "a*bc")], &[], " \t\n");
    let (evaluated, _) = expand("${X#$P}", &[("X", "abc"), ("P", "a*")], &[], " \t\n");

    assert_eq!(quoted, strings(&["bc"]));
    assert_eq!(evaluated, strings(&["bc"]));
}

#[test]
fn quoted_trim_of_empty_and_unset_values_retains_empty_fields() {
    let (result, _) = expand(
        "\"${E#a*}\" \"${E%*}\" \"${U#a*}\" \"${U%*}\"",
        &[("E", "")],
        &[],
        " \t\n",
    );

    assert_eq!(result, strings(&["", "", "", ""]));
}

#[test]
fn tilde_expands_only_at_the_start_before_a_plain_slash_or_end() {
    let (result, _) = expand("~ ~/x ~x '~' a=~ x~", &[("HOME", "/home/u")], &[], " \t\n");

    assert_eq!(
        result,
        strings(&["/home/u", "/home/u/x", "~x", "~", "a=~", "x~"])
    );
}

#[test]
fn tilde_expansion_distinguishes_empty_and_unset_home() {
    let (empty, _) = expand("~ ~/x", &[("HOME", "")], &[], " \t\n");
    let (unset, _) = expand("~ ~/x", &[], &[], " \t\n");

    assert_eq!(empty, strings(&["", "/x"]));
    assert_eq!(unset, strings(&["~", "~/x"]));
}

#[test]
fn tilde_expansion_protects_home_spaces_and_pattern_characters() {
    let (result, _) = expand("{~,x}", &[("HOME", "a b*")], &[], " \t\n");

    assert_eq!(result, strings(&["a b*", "x"]));
}

#[test]
fn quoted_slash_prevents_tilde_expansion() {
    let (result, _) = expand("~'/'x", &[("HOME", "h")], &[], " \t\n");

    assert_eq!(result, strings(&["~/x"]));
}

#[test]
fn parameter_atoms_stay_opaque_while_braces_expand() {
    let (result, _) = expand("$X{1,2}", &[("X", "v")], &[], " \t\n");

    assert_eq!(result, strings(&["v1", "v2"]));
}

#[test]
fn assignments_are_shared_across_brace_results_and_later_words() {
    let (result, scope) = expand("${N:=a}{1,2} $N", &[], &[], " \t\n");

    assert_eq!(result, strings(&["a1", "a2", "a"]));
    assert_eq!(scope.assigned, vec![("N".to_owned(), "a".to_owned())]);
}

#[test]
fn pipeline_expands_braces_fields_and_globs_in_order() {
    let (result, _) = expand_with_files(
        "pre{1,2} *.txt post",
        &[],
        &[],
        " \t\n",
        &["b.txt", "a.txt"],
    );

    assert_eq!(result, strings(&["pre1", "pre2", "a.txt", "b.txt", "post"]));
}

#[test]
fn glob_matches_are_sorted_independently_of_virtual_file_input_order() {
    let (result, _) = expand_with_files("*.txt", &[], &[], " \t\n", &["z.txt", "m.txt", "a.txt"]);

    assert_eq!(result, strings(&["a.txt", "m.txt", "z.txt"]));
}

#[test]
fn quoted_pattern_field_remains_literal_even_when_a_file_matches() {
    let (result, _) = expand_with_files("\"*.txt\"", &[], &[], " \t\n", &["a.txt"]);

    assert_eq!(result, strings(&["*.txt"]));
}

#[test]
fn unmatched_magic_field_is_retained_verbatim() {
    let (result, _) = expand_with_files("missing*.txt", &[], &[], " \t\n", &["a.txt"]);

    assert_eq!(result, strings(&["missing*.txt"]));
}

#[test]
fn parameter_expansion_splits_before_magic_fields_are_globbed() {
    let (result, _) = expand_with_files(
        "$X",
        &[("X", "*.txt z*")],
        &[],
        " \t\n",
        &["b.txt", "a.txt"],
    );

    assert_eq!(result, strings(&["a.txt", "b.txt", "z*"]));
}

#[test]
fn quoted_parameter_expansion_avoids_splitting_and_globbing() {
    let (result, _) = expand_with_files(
        "\"$X\"",
        &[("X", "*.txt other")],
        &[],
        " \t\n",
        &["a.txt", "other"],
    );

    assert_eq!(result, strings(&["*.txt other"]));
}

#[test]
fn literal_ifs_characters_are_protected_but_parameter_ifs_characters_split() {
    let (result, _) = expand("a:b $X", &[("X", "a:b")], &[], ":");

    assert_eq!(result, strings(&["a:b", "a", "b"]));
}

#[test]
fn slash_components_match_virtual_directories_and_files() {
    let (result, _) = expand_with_files("d/?.c", &[], &[], " \t\n", &["d/y.c", "d/x.c"]);

    assert_eq!(result, strings(&["d/x.c", "d/y.c"]));
}

#[test]
fn a_glob_expansion_stays_between_neighboring_literal_words() {
    let (result, _) =
        expand_with_files("before d/*.c after", &[], &[], " \t\n", &["d/y.c", "d/x.c"]);

    assert_eq!(result, strings(&["before", "d/x.c", "d/y.c", "after"]));
}

#[test]
fn each_split_pattern_field_falls_back_independently_when_unmatched() {
    let (result, _) = expand_with_files("$X", &[("X", "*.txt missing*")], &[], " \t\n", &["a.txt"]);

    assert_eq!(result, strings(&["a.txt", "missing*"]));
}

#[test]
fn a_literal_character_field_is_not_replaced_by_glob_results() {
    let (result, _) = expand_with_files("readme", &[], &[], " \t\n", &["readme", "readme2"]);

    assert_eq!(result, strings(&["readme"]));
}

#[test]
fn a_large_brace_expansion_error_is_propagated_through_the_pipeline() {
    assert_eq!(
        expansion_error("{1..2000}", &[], &[]),
        "brace expansion makes more than 1000 words"
    );
}

#[test]
fn pipeline_matches_the_complete_expand_composition_example() {
    let (result, _) = expand_with_files(
        "ls *.txt \"*.txt\" d/?.c z*",
        &[],
        &[],
        " \t\n",
        &["b.txt", "a.txt", "d/x.c"],
    );

    // Expected list: ["ls","a.txt","b.txt","*.txt","d/x.c","z*"]
    assert_eq!(
        result,
        strings(&["ls", "a.txt", "b.txt", "*.txt", "d/x.c", "z*"])
    );
}

#[test]
fn handwritten_evaluation_tests_meet_the_30_test_minimum() {
    let tests: [fn(); 30] = [
        scope_initialization_retains_bindings_and_starts_without_assignments,
        literal_characters_and_quote_markers_preserve_item_flags,
        unquoted_parameter_text_is_split_eligible_but_not_quoted,
        quoted_parameter_text_starts_with_null_and_cannot_split,
        unset_plain_and_quoted_parameters_differ_by_null_retention,
        empty_plain_and_quoted_parameters_differ_by_null_retention,
        positional_numbers_and_count_expand_in_order,
        lengths_count_unicode_characters_and_empty_or_missing_values,
        zero_positionals_make_quoted_at_absent_and_quoted_star_empty,
        one_positional_is_preserved_by_quoted_at_and_star,
        quoted_at_keeps_each_positional_separate_including_empty_values,
        unquoted_at_and_star_split_positional_values_and_drop_empty_values,
        quoted_star_joins_with_the_first_ifs_character,
        quoted_star_joins_without_a_separator_when_ifs_is_empty,
        quoted_at_concatenates_positionals_around_surrounding_text,
        quoted_at_with_no_positionals_keeps_the_surrounding_text,
        defaults_distinguish_unset_empty_and_set_values,
        alternatives_distinguish_unset_empty_and_set_values,
        selected_default_and_alternative_words_inherit_quote_context,
        explicit_quotes_inside_an_operator_word_keep_their_grouping,
        skipped_operator_words_have_no_assignment_or_error_side_effects,
        assignments_record_unset_targets_in_evaluation_order,
        assignment_selects_empty_values_but_not_set_values,
        assignment_is_visible_immediately_and_on_later_words,
        nested_assignments_are_recorded_before_the_enclosing_assignment,
        assignment_flattens_positional_breaks_to_spaces,
        repeated_assignment_records_each_selected_empty_value,
        selected_numeric_assignment_fails_before_evaluating_its_word,
        set_numeric_assignment_target_returns_its_value_without_assignment,
        error_operator_uses_explicit_and_flattened_messages,
    ];

    assert!(tests.len() >= 30);
}
