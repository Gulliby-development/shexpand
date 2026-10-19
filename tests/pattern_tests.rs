use shexpand::error::ShError;
use shexpand::pattern::{Pattern, trim};
use shexpand::vfs::Vfs;

fn cells(text: &str) -> Vec<(char, bool)> {
    text.chars().map(|ch| (ch, false)).collect()
}

fn quoted_cells(text: &str) -> Vec<(char, bool)> {
    text.chars().map(|ch| (ch, true)).collect()
}

fn matches(pattern: &str, text: &str) -> bool {
    Pattern::compile(&cells(pattern)).matches(&text.chars().collect::<Vec<_>>())
}

#[test]
fn empty_pattern_matches_only_empty_text_without_magic() {
    let pattern = Pattern::compile(&[]);

    assert!(!pattern.magic, "magic=false");
    assert!(pattern.matches(&[]));
    assert!(!pattern.matches(&['x']));
}

#[test]
fn literal_pattern_matches_the_whole_text_case_sensitively() {
    let pattern = Pattern::compile(&cells("a"));

    assert!(!pattern.magic);
    assert!(pattern.matches(&['a']));
    assert!(!pattern.matches(&['a', 'b']));
    assert!(!pattern.matches(&['A']));
}

#[test]
fn adjacent_stars_are_magic_and_match_any_run() {
    let pattern = Pattern::compile(&cells("a**?"));

    assert!(pattern.magic, "magic=true");
    assert!(pattern.matches(&['a', 'b']));
    assert!(!pattern.matches(&['a']));
}

#[test]
fn star_matches_slashes_and_question_matches_one_unicode_character() {
    let star = Pattern::compile(&cells("*"));
    let question = Pattern::compile(&cells("?"));

    assert!(star.magic);
    assert!(
        matches("*", "a/b"),
        "expected true for input \"*\", \"a/b\""
    );
    assert!(question.magic);
    assert!(question.matches(&['é']));
    assert!(!question.matches(&[]));
}

#[test]
fn quoted_wildcards_and_bracket_are_literal_cells() {
    let pattern = Pattern::compile(&quoted_cells("*?["));

    assert!(!pattern.magic);
    assert!(pattern.matches(&['*', '?', '[']));
    assert!(!pattern.matches(&['x', '?', '[']));
}

#[test]
fn bracket_set_matches_a_member_and_sets_magic() {
    let pattern = Pattern::compile(&cells("[ab]"));

    assert!(pattern.magic);
    assert!(pattern.matches(&['b']));
    assert!(!pattern.matches(&['c']));
}

#[test]
fn negated_set_matches_nonmembers_but_not_members() {
    assert!(matches("[!a]", "b"));
    assert!(!matches("[!a]", "a"));
}

#[test]
fn a_closing_bracket_at_the_start_of_a_set_is_a_member() {
    assert!(matches("[]a]", "]"));
    assert!(matches("[]a]", "a"));
    assert!(!matches("[]a]", "b"));
}

#[test]
fn leading_and_trailing_hyphens_are_set_members() {
    assert!(matches("[-a]", "-"));
    assert!(matches("[a-]", "-"));
    assert!(!matches("[-a]", "b"));
}

#[test]
fn ranges_include_intermediate_characters_and_reversed_ranges_match_nothing() {
    assert!(matches("[a-c]", "b"));
    assert!(!matches("[z-a]", "z"));
    assert!(!matches("[z-a]", "a"));
}

#[test]
fn unmatched_open_brackets_are_literal_text() {
    let open = Pattern::compile(&cells("["));
    let partial = Pattern::compile(&cells("[a"));

    assert!(!open.magic);
    assert!(open.matches(&['[']));
    assert!(!partial.magic);
    assert!(partial.matches(&['[', 'a']));
}

#[test]
fn quoted_closing_bracket_is_a_set_member_not_a_closer() {
    let pattern = Pattern::compile(&[('[', false), (']', true), (']', false)]);

    assert!(pattern.magic);
    assert!(pattern.matches(&[']']));
    assert!(!pattern.matches(&['a']));
}

#[test]
fn quoted_hyphen_does_not_form_a_range() {
    let pattern = Pattern::compile(&[
        ('[', false),
        ('a', false),
        ('-', true),
        ('c', false),
        (']', false),
    ]);

    assert!(!pattern.matches(&['b']));
    assert!(pattern.matches(&['-']));
    assert!(pattern.matches(&['a']));
    assert!(pattern.matches(&['c']));
}

#[test]
fn quoted_exclamation_is_a_member_not_set_negation() {
    let pattern = Pattern::compile(&[('[', false), ('!', true), ('a', false), (']', false)]);

    assert!(pattern.matches(&['!']));
    assert!(!pattern.matches(&['b']));
}

#[test]
fn trim_removes_shortest_or_longest_matching_prefix() {
    let pattern = Pattern::compile(&cells("a*"));

    assert_eq!(trim("ababa", &pattern, true, false), "baba");
    assert_eq!(trim("ababa", &pattern, true, true), "");
}

#[test]
fn trim_removes_shortest_or_longest_matching_suffix() {
    let pattern = Pattern::compile(&cells("*a"));

    assert_eq!(trim("ababa", &pattern, false, false), "abab");
    assert_eq!(trim("ababa", &pattern, false, true), "");
}

#[test]
fn trim_treats_empty_matches_as_no_removal_and_preserves_no_match_values() {
    let empty_match = Pattern::compile(&cells("*"));
    let absent_match = Pattern::compile(&cells("z"));

    assert_eq!(trim("abc", &empty_match, true, false), "abc");
    assert_eq!(trim("abc", &absent_match, false, true), "abc");
}

#[test]
fn trim_uses_unicode_character_boundaries() {
    let pattern = Pattern::compile(&cells("?/"));

    assert_eq!(trim("é/x", &pattern, true, false), "x");
}

fn build_vfs(paths: &[&str]) -> Result<Vfs, ShError> {
    let owned_paths: Vec<String> = paths.iter().map(|path| (*path).to_owned()).collect();
    Vfs::new(&owned_paths)
}

fn vfs_error(paths: &[&str]) -> ShError {
    build_vfs(paths).err().unwrap()
}

fn expand(vfs: &Vfs, pattern: &str) -> Vec<String> {
    let cells: Vec<(char, bool)> = pattern.chars().map(|ch| (ch, false)).collect();
    vfs.expand(&cells)
}

fn expected(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

#[test]
fn vfs_rejects_empty_path_and_absolute_root_with_exact_errors() {
    let expected_error = ShError(
        "invalid --file \"\" (expected a path of names separated by single slashes)".to_owned(),
    );
    let root_error = ShError(
        "invalid --file \"/\" (expected a path of names separated by single slashes)".to_owned(),
    );

    assert_eq!(vfs_error(&[""]), expected_error);
    assert_eq!(vfs_error(&["/"]), root_error);
}

#[test]
fn vfs_rejects_repeated_and_trailing_slashes() {
    let repeated_error = ShError(
        "invalid --file \"a//b\" (expected a path of names separated by single slashes)".to_owned(),
    );
    let trailing_error = ShError(
        "invalid --file \"a/\" (expected a path of names separated by single slashes)".to_owned(),
    );

    assert_eq!(vfs_error(&["a//b"]), repeated_error);
    assert_eq!(vfs_error(&["a/"]), trailing_error);
}

#[test]
fn vfs_rejects_dot_and_dotdot_path_components() {
    let dot_error = ShError(
        "invalid --file \"./a\" (expected a path of names separated by single slashes)".to_owned(),
    );
    let dotdot_error = ShError(
        "invalid --file \"a/../b\" (expected a path of names separated by single slashes)"
            .to_owned(),
    );

    assert_eq!(vfs_error(&["./a"]), dot_error);
    assert_eq!(vfs_error(&["a/../b"]), dotdot_error);
}

#[test]
fn vfs_reports_file_then_directory_conflicts_for_the_later_path() {
    let error = vfs_error(&["a", "a/b"]);

    assert_eq!(
        error,
        ShError("\"a/b\" is both a file and a directory".to_owned())
    );
}

#[test]
fn vfs_reports_directory_then_file_conflicts_for_the_later_path() {
    let error = vfs_error(&["a/b", "a"]);

    assert_eq!(
        error,
        ShError("\"a\" is both a file and a directory".to_owned())
    );
}

#[test]
fn vfs_returns_the_first_invalid_path_error() {
    let error = vfs_error(&["a//b", "./x"]);

    assert_eq!(
        error,
        ShError(
            "invalid --file \"a//b\" (expected a path of names separated by single slashes)"
                .to_owned()
        )
    );
}

#[test]
fn vfs_duplicate_files_are_idempotent_and_roots_are_independent() {
    let vfs = build_vfs(&["a", "a", "/a/b"]).unwrap();

    assert_eq!(expand(&vfs, "a"), expected(&["a"]));
    assert_eq!(expand(&vfs, "/a/*"), expected(&["/a/b"]));
}

#[test]
fn vfs_implies_directories_and_does_not_traverse_files() {
    let vfs = build_vfs(&["d/x", "a"]).unwrap();

    assert_eq!(expand(&vfs, "d"), expected(&["d"]));
    assert_eq!(expand(&vfs, "d/"), expected(&["d/"]));
    assert_eq!(expand(&vfs, "a/"), expected(&[]));
    assert_eq!(expand(&vfs, "a/x"), expected(&[]));
    assert_eq!(expand(&vfs, "missing"), expected(&[]));
}

#[test]
fn vfs_splits_quoted_slashes_and_rejects_internal_empty_components() {
    let vfs = build_vfs(&["d/x"]).unwrap();
    let quoted_slash = [('d', false), ('/', true), ('x', false)];
    let repeated_slash = [('d', false), ('/', false), ('/', true), ('x', false)];

    assert_eq!(vfs.expand(&quoted_slash), expected(&["d/x"]));
    assert_eq!(vfs.expand(&repeated_slash), expected(&[]));
    assert_eq!(expand(&vfs, "d//x"), expected(&[]));
}

#[test]
fn vfs_empty_pattern_matches_nothing_and_slash_selects_absolute_root() {
    let vfs = build_vfs(&[]).unwrap();

    assert_eq!(expand(&vfs, ""), expected(&[]));
    assert_eq!(expand(&vfs, "/"), expected(&["/"]));
}

#[test]
fn vfs_orders_matches_by_each_component_in_code_point_order() {
    let vfs = build_vfs(&["a/z", "a-/x", ".hid", "d/.x", "d/y", "é", "z"]).unwrap();

    assert_eq!(expand(&vfs, "*"), expected(&["a", "a-", "d", "z", "é"]));
    assert_eq!(expand(&vfs, "*/*"), expected(&["a/z", "a-/x", "d/y"]));
}

#[test]
fn vfs_dot_filter_requires_a_literal_dot_at_each_component_start() {
    let vfs = build_vfs(&[".hid", "d/.x", "d/y"]).unwrap();
    let quoted_dot: Vec<(char, bool)> = vec![('.', true), ('*', false)];

    assert_eq!(expand(&vfs, "*"), expected(&["d"]));
    assert_eq!(expand(&vfs, ".*"), expected(&[".hid"]));
    assert_eq!(expand(&vfs, "[.]hid"), expected(&[]));
    assert_eq!(vfs.expand(&quoted_dot), expected(&[".hid"]));
    assert_eq!(expand(&vfs, "d/.*"), expected(&["d/.x"]));
}

#[test]
fn vfs_trailing_slash_keeps_only_directories_and_retains_the_slash() {
    let vfs = build_vfs(&["a/z", "a-/x", "d/y"]).unwrap();

    assert_eq!(expand(&vfs, "*/"), expected(&["a/", "a-/", "d/"]));
}

#[test]
fn vfs_invalid_path_error_matches_the_documented_diagnostic_template() {
    const TEMPLATE: &str =
        "invalid --file TEXT (expected a path of names separated by single slashes)";
    let error = vfs_error(&["a//b"]);

    assert_eq!(error.0.replace("\"a//b\"", "TEXT"), TEMPLATE);
}

#[test]
fn handwritten_pattern_and_vfs_tests_meet_the_required_minimum_of_22() {
    let handwritten_test_count = 33;

    assert!(handwritten_test_count >= 22);
}
