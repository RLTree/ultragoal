use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    root: PathBuf,
    files: Vec<String>,
}
impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "legibility-modules-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        for (path, text) in files {
            let destination = root.join(path);
            std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
            std::fs::write(destination, text).unwrap();
        }
        Self {
            root,
            files: files.iter().map(|(path, _)| path.to_string()).collect(),
        }
    }
    fn check(&self, roots: &[&str]) -> Vec<String> {
        super::check(
            &self.root,
            &self.files,
            &roots
                .iter()
                .map(|path| path.to_string())
                .collect::<Vec<_>>(),
        )
        .failures
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn resolves_directory_inline_and_explicit_modules() {
    let fixture = Fixture::new(&[
        (
            "src/lib.rs",
            "mod domain; mod inline { #[path = \"leaf.rs\"] mod renamed; }",
        ),
        ("src/domain/mod.rs", "mod leaf;"),
        ("src/domain/leaf.rs", ""),
        ("src/inline/leaf.rs", ""),
    ]);
    assert_eq!(fixture.check(&["src/lib.rs"]), Vec::<String>::new());
}

#[test]
fn cargo_integration_root_and_children_are_exempt_from_partial_factoring() {
    let fixture = Fixture::new(&[
        (
            "tests/roundtrip.rs",
            "#[path = \"roundtrip/leaf.rs\"] mod leaf;",
        ),
        ("tests/roundtrip/leaf.rs", ""),
    ]);
    assert!(fixture.check(&["tests/roundtrip.rs"]).is_empty());
}

#[test]
fn conditional_missing_modules_are_rejected() {
    let fixture = Fixture::new(&[("src/lib.rs", "#[cfg(feature = \"optional\")] mod absent;")]);
    assert!(
        fixture
            .check(&["src/lib.rs"])
            .iter()
            .any(|error| error.starts_with("module_missing:"))
    );
}

#[test]
fn conditional_path_authority_fails_explicitly() {
    let fixture = Fixture::new(&[(
        "src/lib.rs",
        "#[cfg_attr(unix, path = \"other.rs\")] mod name;",
    )]);
    assert!(
        fixture
            .check(&["src/lib.rs"])
            .iter()
            .any(|error| error.contains("conditional_cfg_or_path_attribute_unsupported"))
    );
}

#[test]
fn ambiguous_file_and_directory_modules_are_rejected() {
    let fixture = Fixture::new(&[
        ("src/lib.rs", "mod leaf;"),
        ("src/leaf.rs", ""),
        ("src/leaf/mod.rs", ""),
    ]);
    assert!(
        fixture
            .check(&["src/lib.rs"])
            .iter()
            .any(|error| error.starts_with("module_ambiguous:"))
    );
}

#[test]
fn cycles_and_unreachable_files_are_rejected() {
    let fixture = Fixture::new(&[
        ("src/lib.rs", "mod first;"),
        ("src/first.rs", "#[path = \"second.rs\"] mod second;"),
        ("src/second.rs", "#[path = \"first.rs\"] mod first;"),
        ("src/orphan.rs", ""),
    ]);
    let errors = fixture.check(&["src/lib.rs"]);
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with("module_cycle:"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error == "module_orphan:src/orphan.rs")
    );
}

#[test]
fn partial_factoring_is_a_policy_violation_even_when_rust_can_resolve_it() {
    let fixture = Fixture::new(&[
        ("src/lib.rs", "mod domain;"),
        ("src/domain.rs", "mod leaf;"),
        ("src/domain/leaf.rs", ""),
    ]);
    let errors = fixture.check(&["src/lib.rs"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].starts_with("module_partial_factoring:"));
}

#[test]
fn three_shared_prefix_siblings_require_routing() {
    let fixture = Fixture::new(&[
        (
            "src/lib.rs",
            "mod store_read; mod store_write; mod store_list;",
        ),
        ("src/store_read.rs", ""),
        ("src/store_write.rs", ""),
        ("src/store_list.rs", ""),
    ]);
    assert!(
        fixture
            .check(&["src/lib.rs"])
            .iter()
            .any(|error| error.starts_with("module_residual_prefix:"))
    );
    let integration = Fixture::new(&[
        ("tests/store_read.rs", ""),
        ("tests/store_write.rs", ""),
        ("tests/store_list.rs", ""),
    ]);
    assert!(
        integration
            .check(&[
                "tests/store_read.rs",
                "tests/store_write.rs",
                "tests/store_list.rs"
            ])
            .is_empty()
    );
}

#[test]
fn explicit_paths_must_stay_inside_project() {
    let fixture = Fixture::new(&[("src/lib.rs", "#[path = \"../../outside.rs\"] mod outside;")]);
    assert!(
        fixture
            .check(&["src/lib.rs"])
            .iter()
            .any(|error| error.contains("outside_project"))
    );
}

#[cfg(unix)]
#[test]
fn symlink_modules_are_rejected() {
    let mut fixture = Fixture::new(&[("src/lib.rs", "mod linked;"), ("src/actual.rs", "")]);
    std::os::unix::fs::symlink("actual.rs", fixture.root.join("src/linked.rs")).unwrap();
    fixture.files.push("src/linked.rs".to_string());
    assert!(
        fixture
            .check(&["src/lib.rs"])
            .iter()
            .any(|error| error.contains("symlink_forbidden"))
    );
}

#[test]
fn nonstandard_cargo_root_resolves_siblings_without_filename_directory() {
    let fixture = Fixture::new(&[("bin/entry.rs", "mod leaf;"), ("bin/leaf.rs", "")]);
    assert!(fixture.check(&["bin/entry.rs"]).is_empty());
}

#[test]
fn nested_inline_path_override_uses_its_declared_directory() {
    let fixture = Fixture::new(&[
        (
            "src/lib.rs",
            "#[path = \"storage\"] mod domain { mod leaf; }",
        ),
        ("src/storage/leaf.rs", ""),
    ]);
    assert!(fixture.check(&["src/lib.rs"]).is_empty());
}

#[test]
fn safe_source_path_carries_validated_relative_path_and_named_refusal() {
    use super::resolution::{SourcePathError, safe_path};
    use std::path::Path;
    let fixture = Fixture::new(&[("src/lib.rs", "")]);
    let valid = safe_path(&fixture.root, Path::new("src/./lib.rs")).unwrap();
    assert_eq!(valid.as_path(), Path::new("src/lib.rs"));
    for path in ["../outside.rs", "/outside.rs"] {
        assert!(matches!(
            safe_path(&fixture.root, Path::new(path)),
            Err(SourcePathError::OutsideProject)
        ));
    }
    assert_eq!(
        SourcePathError::OutsideProject.to_string(),
        "outside_project"
    );
    assert_eq!(
        SourcePathError::SymlinkForbidden.to_string(),
        "symlink_forbidden"
    );
    assert_eq!(SourcePathError::NonUtf8Path.to_string(), "non_utf8_path");
}
