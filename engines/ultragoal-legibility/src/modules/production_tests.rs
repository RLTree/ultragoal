use super::production_paths;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    root: PathBuf,
    paths: Vec<String>,
}
impl Fixture {
    fn new(sources: &[(&str, &str)]) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "legibility-production-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        for (path, source) in sources {
            let destination = root.join(path);
            std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
            std::fs::write(destination, source).unwrap();
        }
        Self {
            root,
            paths: sources.iter().map(|(path, _)| path.to_string()).collect(),
        }
    }
    fn analyze(&self, entries: &[&str]) -> (BTreeSet<String>, Vec<String>) {
        let sources = production_paths(
            &self.root,
            &self.paths,
            &entries
                .iter()
                .map(|entry| entry.to_string())
                .collect::<Vec<_>>(),
        );
        (sources.test_only, sources.failures)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn externally_test_owned_module_and_descendants_are_exempt() {
    let fixture = Fixture::new(&[
        ("src/lib.rs", "#[cfg(test)] mod tests;"),
        ("src/tests/mod.rs", "mod helper;"),
        ("src/tests/helper.rs", ""),
    ]);
    let (test_only, failures) = fixture.analyze(&["src/lib.rs"]);
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        test_only,
        BTreeSet::from(["src/tests/mod.rs".into(), "src/tests/helper.rs".into()])
    );
}

#[test]
fn test_filename_without_test_ownership_stays_production() {
    let fixture = Fixture::new(&[("src/lib.rs", "mod tests;"), ("src/tests.rs", "")]);
    assert!(fixture.analyze(&["src/lib.rs"]).0.is_empty());
}

#[test]
fn mixed_production_and_test_owners_prevent_exemption() {
    let fixture = Fixture::new(&[
        ("src/lib.rs", "mod runtime; #[cfg(test)] mod tests;"),
        ("src/runtime.rs", "#[path = \"shared.rs\"] mod shared;"),
        ("src/tests.rs", "#[path = \"shared.rs\"] mod shared;"),
        ("src/shared.rs", ""),
    ]);
    let (test_only, failures) = fixture.analyze(&["src/lib.rs"]);
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(test_only, BTreeSet::from(["src/tests.rs".into()]));
}

#[test]
fn cargo_integration_entrypoint_owns_external_support_as_test_only() {
    let fixture = Fixture::new(&[
        ("tests/contract.rs", "#[path = \"support.rs\"] mod support;"),
        ("tests/support.rs", ""),
    ]);
    let (test_only, failures) = fixture.analyze(&["tests/contract.rs"]);
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(test_only.len(), 2);
}

#[test]
fn cfg_boolean_expressions_preserve_unknown_production_branches() {
    let fixture = Fixture::new(&[
        (
            "src/lib.rs",
            "#[cfg(all(test, unix))] mod only; #[cfg(any(test, feature = \"enabled\"))] mod mixed; #[cfg(not(test))] mod prod;",
        ),
        ("src/only.rs", ""),
        ("src/mixed.rs", ""),
        ("src/prod.rs", ""),
    ]);
    let (test_only, failures) = fixture.analyze(&["src/lib.rs"]);
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(test_only, BTreeSet::from(["src/only.rs".into()]));
}

#[test]
fn inline_test_ownership_and_path_overrides_propagate() {
    let fixture = Fixture::new(&[
        (
            "src/lib.rs",
            "#[cfg(test)] #[path = \"test_support\"] mod checks { mod leaf; }",
        ),
        ("src/test_support/leaf.rs", ""),
    ]);
    let (test_only, failures) = fixture.analyze(&["src/lib.rs"]);
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        test_only,
        BTreeSet::from(["src/test_support/leaf.rs".into()])
    );
}

#[test]
fn file_level_cfg_test_is_respected() {
    let fixture = Fixture::new(&[
        ("src/lib.rs", "mod support;"),
        ("src/support.rs", "#![cfg(test)]"),
    ]);
    assert_eq!(
        fixture.analyze(&["src/lib.rs"]).0,
        BTreeSet::from(["src/support.rs".into()])
    );
}

#[test]
fn cfg_malformed_and_conditional_authority_fail_explicitly() {
    let fixture = Fixture::new(&[
        (
            "src/lib.rs",
            "#[cfg(not(test, unix))] mod bad; #[cfg_attr(unix, path = \"bad.rs\")] mod rewrite;",
        ),
        ("src/bad.rs", ""),
    ]);
    let (test_only, failures) = fixture.analyze(&["src/lib.rs"]);
    assert!(test_only.is_empty());
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("cfg_not_arity_invalid"))
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("conditional_cfg_or_path_attribute_unsupported"))
    );
}

#[test]
fn orphan_and_test_cycle_cannot_gain_test_exemptions() {
    let fixture = Fixture::new(&[
        ("src/lib.rs", "#[cfg(test)] mod first;"),
        ("src/first.rs", "#[path = \"second.rs\"] mod second;"),
        ("src/second.rs", "#[path = \"first.rs\"] mod first;"),
        ("src/orphan.rs", ""),
    ]);
    let (test_only, failures) = fixture.analyze(&["src/lib.rs"]);
    assert!(test_only.is_empty());
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("production_source_module_cycle"))
    );
}

#[test]
fn unknown_platform_alone_is_not_a_test_exemption() {
    let fixture = Fixture::new(&[
        (
            "src/lib.rs",
            "#[cfg(target_os = \"never_seen\")] mod platform;",
        ),
        ("src/platform.rs", ""),
    ]);
    assert!(fixture.analyze(&["src/lib.rs"]).0.is_empty());
}
