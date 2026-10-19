use shexpand::atom::{Atom, Kind, Param};
use shexpand::braces::expand;
use shexpand::error::ShError;

fn ch(ch: char, quoted: bool) -> Atom {
    Atom::Ch { ch, quoted }
}

fn plain(text: &str) -> Vec<Atom> {
    text.chars().map(|character| ch(character, false)).collect()
}

fn assert_plain_results(input: &str, expected: &[&str]) {
    assert_eq!(
        expand(&plain(input)).unwrap(),
        expected.iter().map(|text| plain(text)).collect::<Vec<_>>()
    );
}

fn parameter() -> Atom {
    Atom::Param(Box::new(Param {
        kind: Kind::Plain,
        name: "X".to_owned(),
        colon: false,
        longest: false,
        word: plain("{a,b}"),
        text: "$X".to_owned(),
        col: 7,
        quoted: true,
    }))
}

#[test]
fn empty_atoms_expand_to_one_empty_word() {
    assert_eq!(expand(&[]), Ok(vec![vec![]]));
}

#[test]
fn literal_atoms_are_preserved() {
    let input = vec![ch('a', false), ch('b', true), Atom::Quote];

    assert_eq!(expand(&input), Ok(vec![input]));
}

#[test]
fn parameter_is_opaque_and_retains_all_metadata() {
    let parameter = parameter();
    let mut input = vec![parameter.clone()];
    input.extend(plain("{1,2}"));
    let mut first = vec![parameter.clone()];
    first.extend(plain("1"));
    let mut second = vec![parameter];
    second.extend(plain("2"));

    assert_eq!(expand(&input), Ok(vec![first, second]));
}

#[test]
fn standalone_quote_marker_is_retained() {
    assert_eq!(expand(&[Atom::Quote]), Ok(vec![vec![Atom::Quote]]));
}

#[test]
fn quote_marker_in_an_alternative_is_retained() {
    assert_eq!(
        expand(&[
            ch('{', false),
            Atom::Quote,
            ch(',', false),
            ch('a', false),
            ch('}', false)
        ]),
        Ok(vec![vec![Atom::Quote], plain("a")])
    );
}

#[test]
fn quoted_opening_brace_does_not_start_an_alternative() {
    let input = vec![
        ch('{', true),
        ch('a', false),
        ch(',', false),
        ch('b', false),
        ch('}', false),
    ];

    assert_eq!(expand(&input), Ok(vec![input.clone()]));
}

#[test]
fn quoted_comma_does_not_split_an_alternative() {
    let input = vec![
        ch('{', false),
        ch('a', false),
        ch(',', true),
        ch('b', false),
        ch('}', false),
    ];

    assert_eq!(expand(&input), Ok(vec![input.clone()]));
}

#[test]
fn quoted_closing_brace_does_not_close_an_alternative() {
    let input = vec![
        ch('{', false),
        ch('a', false),
        ch(',', false),
        ch('b', false),
        ch('}', true),
    ];

    assert_eq!(expand(&input), Ok(vec![input.clone()]));
}

#[test]
fn alternative_character_keeps_its_quote_flag() {
    assert_eq!(
        expand(&[
            ch('{', false),
            ch('a', true),
            ch(',', false),
            ch('b', false),
            ch('}', false),
        ]),
        Ok(vec![vec![ch('a', true)], plain("b")])
    );
}

#[test]
fn initial_adjacent_empty_braces_are_literal() {
    assert_plain_results("{}{a,b}", &["{}a", "{}b"]);
}

#[test]
fn early_closer_before_comma_is_ordinary_text() {
    assert_plain_results("{a}b,c}", &["a}b", "c"]);
}

#[test]
fn dots_followed_by_closer_do_not_qualify_early_closer() {
    assert_plain_results("{a..}b,c}", &["a..}b", "c"]);
}

#[test]
fn unmatched_outer_brace_allows_inner_alternative() {
    assert_plain_results("{a{b,c}d}", &["{abd}", "{acd}"]);
}

#[test]
fn invalid_earlier_candidate_does_not_hide_later_alternative() {
    assert_plain_results("{x}{a,b}", &["{x}a", "{x}b"]);
}

#[test]
fn suffix_is_scanned_as_new_text_after_each_choice() {
    assert_plain_results("a{b,c}{}{d,e}", &["ab{}d", "ab{}e", "ac{}d", "ac{}e"]);
}

#[test]
fn alternatives_expand_between_literal_affixes() {
    assert_plain_results("a{b,c}d", &["abd", "acd"]);
}

#[test]
fn alternatives_keep_empty_items() {
    assert_plain_results("{,a,}", &["", "a", ""]);
}

#[test]
fn alternatives_preserve_duplicates() {
    assert_plain_results("{a,a}", &["a", "a"]);
}

#[test]
fn nested_alternatives_use_item_major_product_order() {
    assert_plain_results("{a,{b,c}}{1,2}", &["a1", "a2", "b1", "b2", "c1", "c2"]);
}

#[test]
fn numeric_range_counts_up_by_one() {
    assert_plain_results("{1..3}", &["1", "2", "3"]);
}

#[test]
fn numeric_range_counts_down_and_only_emits_landed_steps() {
    assert_plain_results("{5..0..2}", &["5", "3", "1"]);
}

#[test]
fn numeric_range_steps_across_zero() {
    assert_plain_results("{-1..1}", &["-1", "0", "1"]);
}

#[test]
fn numeric_range_with_equal_endpoints_emits_one_value() {
    assert_plain_results("{2..2}", &["2"]);
}

#[test]
fn numeric_range_pads_zero_prefixed_endpoints() {
    assert_plain_results("f{08..10}.txt", &["f08.txt", "f09.txt", "f10.txt"]);
}

#[test]
fn numeric_range_padding_counts_width_after_a_negative_sign() {
    assert_plain_results("{-02..2}", &["-02", "-01", "000", "001", "002"]);
}

#[test]
fn descending_numeric_range_pads_after_a_negative_sign() {
    assert_plain_results("{2..-02..2}", &["002", "000", "-02"]);
}

#[test]
fn numeric_range_handles_nine_digit_signed_endpoints() {
    assert_plain_results(
        "{-999999999..999999999..1999999998}",
        &["-999999999", "999999999"],
    );
}

#[test]
fn numeric_range_accepts_a_forty_digit_step() {
    assert_plain_results("{1..3..9999999999999999999999999999999999999999}", &["1"]);
}

#[test]
fn lowercase_letter_range_steps_forward() {
    assert_plain_results("{a..e..2}", &["a", "c", "e"]);
}

#[test]
fn uppercase_letter_range_steps_backward() {
    assert_plain_results("{D..A..2}", &["D", "B"]);
}

#[test]
fn equal_letter_range_emits_one_letter() {
    assert_plain_results("{z..z}", &["z"]);
}

#[test]
fn malformed_integer_endpoint_and_range_shapes_stay_literal() {
    assert_plain_results("{+1..2}", &["{+1..2}"]);
    assert_plain_results("{1000000000..2}", &["{1000000000..2}"]);
    assert_plain_results("{..2}", &["{..2}"]);
    assert_plain_results("{1..}", &["{1..}"]);
    assert_plain_results("{1..2..3..4}", &["{1..2..3..4}"]);
}

#[test]
fn malformed_letter_endpoints_stay_literal() {
    assert_plain_results("{a..Z}", &["{a..Z}"]);
    assert_plain_results("{é..ê}", &["{é..ê}"]);
    assert_plain_results("{aa..bb}", &["{aa..bb}"]);
}

#[test]
fn malformed_steps_stay_literal() {
    assert_plain_results("{1..3..0}", &["{1..3..0}"]);
    assert_plain_results("{1..3..01}", &["{1..3..01}"]);
    assert_plain_results("{1..3..-1}", &["{1..3..-1}"]);
    assert_plain_results("{1..3..+1}", &["{1..3..+1}"]);
}

#[test]
fn protected_or_opaque_range_parts_prevent_range_expansion() {
    let quoted_dot = vec![
        ch('{', false),
        ch('1', false),
        ch('.', true),
        ch('.', false),
        ch('3', false),
        ch('}', false),
    ];
    assert_eq!(expand(&quoted_dot), Ok(vec![quoted_dot.clone()]));

    let quote_interrupt = vec![
        ch('{', false),
        ch('1', false),
        Atom::Quote,
        ch('.', false),
        ch('.', false),
        ch('3', false),
        ch('}', false),
    ];
    assert_eq!(expand(&quote_interrupt), Ok(vec![quote_interrupt.clone()]));

    let param_interrupt = vec![
        ch('{', false),
        ch('1', false),
        parameter(),
        ch('.', false),
        ch('.', false),
        ch('3', false),
        ch('}', false),
    ];
    assert_eq!(expand(&param_interrupt), Ok(vec![param_interrupt.clone()]));
}

#[test]
fn invalid_range_candidate_does_not_hide_later_alternative() {
    assert_plain_results("{1..3..0}{a,b}", &["{1..3..0}a", "{1..3..0}b"]);
}

#[test]
fn numeric_range_at_exact_word_limit_is_materialized() {
    let result = expand(&plain("{1..1000}")).unwrap();

    assert_eq!(result.len(), 1000);
    assert_eq!(result[0], plain("1"));
    assert_eq!(result[999], plain("1000"));
}

#[test]
fn numeric_range_above_word_limit_returns_exact_error() {
    assert_eq!(
        expand(&plain("{1..1001}")),
        Err(ShError(
            "brace expansion makes more than 1000 words".to_owned()
        ))
    );
    assert_eq!(
        expand(&plain("{1..2000}")),
        Err(ShError(
            "brace expansion makes more than 1000 words".to_owned()
        ))
    );
}

#[test]
fn cartesian_product_at_exact_word_limit_preserves_order() {
    let result = expand(&plain("{1..10}{1..100}")).unwrap();

    assert_eq!(result.len(), 1000);
    assert_eq!(result[0], plain("11"));
    assert_eq!(result[99], plain("1100"));
    assert_eq!(result[100], plain("21"));
    assert_eq!(result[999], plain("10100"));
}

#[test]
fn cartesian_product_above_word_limit_returns_exact_error() {
    assert_eq!(
        expand(&plain("{1..10}{1..101}")),
        Err(ShError(
            "brace expansion makes more than 1000 words".to_owned()
        ))
    );
}

#[test]
fn nested_alternative_sum_at_exact_word_limit_preserves_order() {
    let result = expand(&plain("{{1..500},{1..500}}")).unwrap();

    assert_eq!(result.len(), 1000);
    assert_eq!(result[0], plain("1"));
    assert_eq!(result[499], plain("500"));
    assert_eq!(result[500], plain("1"));
    assert_eq!(result[999], plain("500"));
}

#[test]
fn nested_alternative_sum_above_word_limit_returns_exact_error() {
    assert_eq!(
        expand(&plain("{{1..500},{1..501}}")),
        Err(ShError(
            "brace expansion makes more than 1000 words".to_owned()
        ))
    );
}

#[test]
fn nested_alternatives_allow_exactly_eight_levels() {
    assert_plain_results(
        "{a,{a,{a,{a,{a,{a,{a,{a,b}}}}}}}}",
        &["a", "a", "a", "a", "a", "a", "a", "a", "b"],
    );
}

#[test]
fn ninth_nested_alternative_returns_exact_depth_error() {
    assert_eq!(
        expand(&plain("{a,{a,{a,{a,{a,{a,{a,{a,{a,b}}}}}}}}}")),
        Err(ShError("nesting is deeper than 8".to_owned()))
    );
}
