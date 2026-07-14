use super::fixture::SourceRoot;

#[test]
fn inline_test_items_are_removed_without_hiding_adjacent_production() {
    let root = SourceRoot::new("production-source-inline-test");
    root.write(
        "validator/src/lib.rs",
        "pub fn production_marker() {}\n\
         #[cfg(test)]\n\
         mod tests { fn hidden_raw_marker() { let raw_path = \"/tmp/test\"; } }\n\
         pub fn after_test_marker() {}\n",
    );
    let set = production_set(&root);
    assert!(set.failures.is_empty(), "{:?}", set.failures);
    let text = source_text(&set, "validator/src/lib.rs");
    assert!(text.contains("production_marker"));
    assert!(text.contains("after_test_marker"));
    assert!(!text.contains("hidden_raw_marker"));
    syn::parse_file(text).expect("filtered production syntax");
}

#[test]
fn conventional_and_path_module_chains_inherit_test_only_ownership() {
    let root = SourceRoot::new("production-source-module-chain");
    root.write(
        "validator/src/lib.rs",
        "#[cfg(test)]\nmod checks;\nmod product;\n",
    );
    root.write(
        "validator/src/checks.rs",
        "#[path = \"checks/nested.rs\"]\nmod nested;\n",
    );
    root.write(
        "validator/src/checks/nested.rs",
        "#[path = \"leaf.rs\"]\nmod leaf;\n",
    );
    root.write("validator/src/checks/leaf.rs", "fn test_leaf_marker() {}\n");
    root.write(
        "validator/src/product.rs",
        "pub fn production_marker() {}\n",
    );
    let set = production_set(&root);
    assert!(set.failures.is_empty(), "{:?}", set.failures);
    for hidden in [
        "validator/src/checks.rs",
        "validator/src/checks/nested.rs",
        "validator/src/checks/leaf.rs",
    ] {
        assert!(
            set.sources.iter().all(|source| source.relative != hidden),
            "{hidden}: {:?}",
            set.sources
        );
    }
    assert!(
        set.sources
            .iter()
            .any(|source| source.relative == "validator/src/product.rs")
    );
}

#[test]
fn mixed_test_and_production_owners_keep_the_shared_file_in_production() {
    let root = SourceRoot::new("production-source-mixed-owners");
    root.write(
        "validator/src/lib.rs",
        "#[cfg(test)]\n#[path = \"shared.rs\"]\nmod test_shared;\n\
         #[path = \"shared.rs\"]\nmod product_shared;\n",
    );
    root.write("validator/src/shared.rs", "pub fn shared_marker() {}\n");
    let set = production_set(&root);
    assert!(set.failures.is_empty(), "{:?}", set.failures);
    assert!(
        set.sources
            .iter()
            .any(|source| source.relative == "validator/src/shared.rs")
    );
}

#[test]
fn malformed_cfg_path_and_module_cycles_fail_closed() {
    let root = SourceRoot::new("production-source-fail-closed");
    root.write(
        "validator/src/lib.rs",
        "#[cfg(test, unix)]\nmod hidden;\n#[path = concat!(\"x.rs\")]\nmod x;\n\
         #[path = \"a.rs\"]\nmod a;\n",
    );
    root.write("validator/src/hidden.rs", "pub fn hidden_marker() {}\n");
    root.write("validator/src/x.rs", "pub fn malformed_path_marker() {}\n");
    root.write("validator/src/a.rs", "#[path = \"b.rs\"]\nmod b;\n");
    root.write("validator/src/b.rs", "#[path = \"a.rs\"]\nmod a;\n");
    let set = production_set(&root);
    for marker in [
        "production_source_cfg_invalid:validator/src/lib.rs",
        "production_source_path_invalid:validator/src/lib.rs",
        "production_source_module_cycle:validator/src/a.rs",
    ] {
        assert!(
            set.failures
                .iter()
                .any(|failure| failure.starts_with(marker)),
            "{marker}: {:?}",
            set.failures
        );
    }
    for retained in [
        "validator/src/hidden.rs",
        "validator/src/x.rs",
        "validator/src/a.rs",
        "validator/src/b.rs",
    ] {
        assert!(
            set.sources.iter().any(|source| source.relative == retained),
            "{retained}: {:?}",
            set.sources
        );
    }
}

#[test]
fn cfg_that_can_compile_without_test_remains_production() {
    let root = SourceRoot::new("production-source-mixed-cfg");
    root.write(
        "validator/src/lib.rs",
        "#[cfg(any(test, unix))]\npub fn mixed_cfg_marker() {}\n",
    );
    let set = production_set(&root);
    assert!(set.failures.is_empty(), "{:?}", set.failures);
    assert!(source_text(&set, "validator/src/lib.rs").contains("mixed_cfg_marker"));
}

fn production_set(root: &SourceRoot) -> super::super::production_source::ProductionSourceSet {
    let inventory =
        super::super::inventory::capture_once_for_test(root.path()).expect("governed inventory");
    super::super::production_source::production_sources(&inventory)
}

fn source_text<'a>(
    set: &'a super::super::production_source::ProductionSourceSet,
    relative: &str,
) -> &'a str {
    let source = set
        .sources
        .iter()
        .find(|source| source.relative == relative)
        .expect("production source");
    std::str::from_utf8(&source.bytes).expect("production utf8")
}
