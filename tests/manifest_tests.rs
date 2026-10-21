const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn manifest_denies_the_four_panicking_constructs_in_a_lints_table() {
    let normalized = MANIFEST.replace("\r\n", "\n");
    assert!(normalized.ends_with(
        "[lints.clippy]\nunreachable = \"deny\"\nunwrap_used = \"deny\"\nexpect_used = \"deny\"\npanic = \"deny\"\n"
    ));
}

#[test]
fn manifest_keeps_the_package_and_an_empty_dependencies_table() {
    let normalized = MANIFEST.replace("\r\n", "\n");
    assert!(normalized.starts_with(
        "[package]\nname = \"shexpand\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n\n[lints.clippy]\n"
    ));
}

#[test]
fn library_and_binary_sources_contain_no_panicking_construct() {
    let source_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut source_files = 0;

    for entry in std::fs::read_dir(&source_dir).expect("failed to read src directory") {
        let path = entry.expect("failed to read src directory entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }

        source_files += 1;
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

        for construct in ["unreachable!(", ".unwrap()", ".expect(", "panic!("] {
            assert!(
                !source.contains(construct),
                "{} contains forbidden construct {construct}",
                path.display()
            );
        }
    }

    assert!(
        source_files >= 20,
        "expected at least 20 Rust source files in src, found {source_files}"
    );
}
