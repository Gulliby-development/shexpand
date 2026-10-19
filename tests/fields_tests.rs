use shexpand::fields::{Item, split};

fn split_char(ch: char) -> Item {
    Item::Ch {
        ch,
        quoted: false,
        split: true,
    }
}

fn literal_char(ch: char) -> Item {
    Item::Ch {
        ch,
        quoted: false,
        split: false,
    }
}

fn quoted_char(ch: char) -> Item {
    Item::Ch {
        ch,
        quoted: true,
        split: false,
    }
}

#[test]
fn literal_whitespace_is_retained() {
    assert_eq!(
        split(
            &[literal_char('a'), literal_char(' '), literal_char('b')],
            " "
        ),
        vec![vec![('a', false), (' ', false), ('b', false)]]
    );
}

#[test]
fn quoted_whitespace_is_retained_with_its_flag() {
    assert_eq!(
        split(&[quoted_char('a'), quoted_char(' '), quoted_char('b')], " "),
        vec![vec![('a', true), (' ', true), ('b', true)]]
    );
}

#[test]
fn quoted_and_split_flags_are_independent() {
    assert_eq!(
        split(
            &[Item::Ch {
                ch: ':',
                quoted: true,
                split: true,
            }],
            ":"
        ),
        vec![vec![]]
    );
}

#[test]
fn empty_ifs_disables_splitting() {
    assert_eq!(
        split(
            &[
                split_char('a'),
                split_char(' '),
                split_char('b'),
                split_char(':'),
                split_char('c')
            ],
            ""
        ),
        vec![vec![
            ('a', false),
            (' ', false),
            ('b', false),
            (':', false),
            ('c', false)
        ]]
    );
}

#[test]
fn nonmember_ifs_character_is_retained() {
    assert_eq!(
        split(&[split_char('a'), split_char(':'), split_char('b')], " "),
        vec![vec![('a', false), (':', false), ('b', false)]]
    );
}

#[test]
fn unicode_ifs_character_separates_fields() {
    assert_eq!(
        split(&[split_char('a'), split_char('λ'), split_char('b')], "λ"),
        vec![vec![('a', false)], vec![('b', false)]]
    );
}

#[test]
fn carriage_return_can_be_a_non_whitespace_delimiter() {
    assert_eq!(
        split(&[split_char('a'), split_char('\r'), split_char('b')], "\r"),
        vec![vec![('a', false)], vec![('b', false)]]
    );
}

#[test]
fn no_items_make_no_fields() {
    assert_eq!(split(&[], " \t\n"), Vec::<Vec<(char, bool)>>::new());
}

#[test]
fn one_null_retains_an_empty_field() {
    assert_eq!(split(&[Item::Null], " \t\n"), vec![vec![]]);
}

#[test]
fn repeated_nulls_still_retain_one_empty_field() {
    assert_eq!(split(&[Item::Null, Item::Null], " \t\n"), vec![vec![]]);
}

#[test]
fn empty_breaks_do_not_make_fields() {
    assert_eq!(
        split(&[Item::Break, Item::Break], " \t\n"),
        Vec::<Vec<(char, bool)>>::new()
    );
}

#[test]
fn nulls_on_each_side_of_break_retain_two_empty_fields() {
    assert_eq!(
        split(&[Item::Null, Item::Break, Item::Null], " \t\n"),
        vec![vec![], vec![]]
    );
}

#[test]
fn break_forces_a_boundary_and_retains_character_flags() {
    assert_eq!(
        split(&[literal_char('a'), Item::Break, quoted_char('b')], " \t\n"),
        vec![vec![('a', false)], vec![('b', true)]]
    );
}

#[test]
fn whitespace_after_null_ends_the_retained_empty_field() {
    assert_eq!(split(&[Item::Null, split_char(' ')], " \t\n"), vec![vec![]]);
}

#[test]
fn ifs_whitespace_runs_collapse_around_fields() {
    assert_eq!(
        split(
            &[
                split_char(' '),
                split_char('\t'),
                split_char('\n'),
                split_char('a'),
                split_char(' '),
                split_char(' '),
                split_char('b'),
                split_char('\t'),
                split_char('\n')
            ],
            " \t\n"
        ),
        vec![vec![('a', false)], vec![('b', false)]]
    );
}

#[test]
fn whitespace_only_input_has_no_fields() {
    assert_eq!(
        split(
            &[split_char(' '), split_char('\t'), split_char('\n')],
            " \t\n"
        ),
        Vec::<Vec<(char, bool)>>::new()
    );
}

#[test]
fn carriage_return_is_retained_by_default_ifs() {
    assert_eq!(
        split(
            &[split_char('a'), split_char('\r'), split_char('b')],
            " \t\n"
        ),
        vec![vec![('a', false), ('\r', false), ('b', false)]]
    );
}

#[test]
fn final_delimiter_does_not_add_an_empty_field() {
    assert_eq!(
        split(&[split_char('a'), split_char(':')], ":"),
        vec![vec![('a', false)]]
    );
}

#[test]
fn repeated_final_delimiters_add_only_the_interior_empty_field() {
    assert_eq!(
        split(&[split_char('a'), split_char(':'), split_char(':')], ":"),
        vec![vec![('a', false)], vec![]]
    );
}

#[test]
fn leading_delimiter_adds_an_empty_field() {
    assert_eq!(
        split(&[split_char(':'), split_char('a')], ":"),
        vec![vec![], vec![('a', false)]]
    );
}

#[test]
fn adjacent_delimiters_add_one_empty_field_between_them() {
    assert_eq!(
        split(&[split_char(':'), split_char(':')], ":"),
        vec![vec![], vec![]]
    );
}

#[test]
fn delimiter_after_break_adds_an_empty_field() {
    assert_eq!(split(&[Item::Break, split_char(':')], ":"), vec![vec![]]);
}

#[test]
fn mixed_whitespace_and_delimiters_split_as_expected() {
    assert_eq!(
        split(
            &[
                split_char(' '),
                split_char('a'),
                split_char(' '),
                split_char(':'),
                split_char(' '),
                split_char('b'),
                split_char(':'),
                split_char(':'),
                split_char('c'),
                split_char(' '),
                split_char(' ')
            ],
            " :"
        ),
        vec![
            vec![('a', false)],
            vec![('b', false)],
            vec![],
            vec![('c', false)]
        ]
    );
}

#[test]
fn trailing_mixed_whitespace_boundary_adds_no_extra_field() {
    assert_eq!(
        split(
            &[
                split_char('a'),
                split_char(' '),
                split_char(':'),
                split_char(' ')
            ],
            " :"
        ),
        vec![vec![('a', false)]]
    );
}

#[test]
fn leading_mixed_boundary_retains_one_empty_field() {
    assert_eq!(
        split(
            &[
                split_char(' '),
                split_char(':'),
                split_char(' '),
                split_char('a')
            ],
            " :"
        ),
        vec![vec![], vec![('a', false)]]
    );
}

#[test]
fn whitespace_between_delimiters_does_not_add_an_extra_empty_field() {
    assert_eq!(
        split(
            &[
                split_char('a'),
                split_char(':'),
                split_char(' '),
                split_char(':'),
                split_char('b')
            ],
            " :"
        ),
        vec![vec![('a', false)], vec![], vec![('b', false)]]
    );
}
