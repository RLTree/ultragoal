use super::*;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const STATIC_REL: &str = "docs/generated/observability/command-inventory.json";

fn root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    fs::create_dir_all(&root).expect("root");
    root
}

fn write_law_rows(root: &Path) {
    for (rel, key, row_key) in [
        ("templates/agent-standards/enforcement.json", "rows", "id"),
        (
            "docs/source-obligation-matrix.json",
            "obligations",
            "obligation_id",
        ),
        (
            "docs/foundational-law-traceability.json",
            "entries",
            "obligation_id",
        ),
    ] {
        let value = match (key, row_key) {
            ("rows", "id") => json!({"rows": [{"id": super::super::LAW}]}),
            ("obligations", "obligation_id") => {
                json!({"obligations": [{"obligation_id": super::super::LAW}]})
            }
            ("entries", "obligation_id") => {
                json!({"entries": [{"obligation_id": super::super::LAW}]})
            }
            _ => unreachable!("fixed law row"),
        };
        crate::json_boundary::write_json(&root.join(rel), &value).expect("law row");
    }
    fs::create_dir_all(root.join("fixtures/mandatory-law-surfaces/valid"))
        .expect("fixture directory");
    fs::write(
        root.join(format!(
            "fixtures/mandatory-law-surfaces/valid/{}.json",
            super::super::LAW
        )),
        "{}",
    )
    .expect("fixture");
}

fn metadata_snapshot(root: &Path) -> Vec<(PathBuf, u64, bool)> {
    let mut rows = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .map(Result::unwrap)
        .map(|entry| {
            let metadata = fs::symlink_metadata(entry.path()).expect("metadata");
            (
                entry
                    .path()
                    .strip_prefix(root)
                    .expect("relative")
                    .to_path_buf(),
                metadata.len(),
                metadata.file_type().is_symlink(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

#[test]
fn registry_preserves_law_checks_but_never_accepts_a_static_successor_catalog() {
    let root = root("observe-registry-unavailable");
    let mut failures = Vec::new();
    check(&root, &mut failures);
    assert!(failures.contains(&SUCCESSOR_CATALOG_UNAVAILABLE.to_string()));
    assert!(
        failures
            .iter()
            .any(|row| row.starts_with("observability_missing_"))
    );

    write_law_rows(&root);
    failures.clear();
    check(&root, &mut failures);
    assert_eq!(failures, vec![SUCCESSOR_CATALOG_UNAVAILABLE.to_string()]);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn command_inventory_bytes_paths_and_absence_have_one_non_echoing_result() {
    let root = root("observe-static-inventory-bait");
    let path = root.join(STATIC_REL);
    fs::create_dir_all(path.parent().expect("parent")).expect("parent");
    let expected = vec![SUCCESSOR_CATALOG_UNAVAILABLE.to_string()];

    assert_eq!(command_inventory_failures(&root), expected);
    fs::write(&path, "SECRET_CANARY").expect("bait");
    assert_eq!(command_inventory_failures(&root), expected);
    fs::write(&path, [0xff, 0xfe]).expect("invalid bait");
    assert_eq!(command_inventory_failures(&root), expected);
    fs::remove_file(&path).expect("remove bait");
    assert_eq!(command_inventory_failures(&root), expected);
    assert!(!expected.join("\n").contains("SECRET_CANARY"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[cfg(unix)]
#[test]
fn static_symlink_and_fifo_are_unread_and_registry_checks_write_nothing() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let root = root("observe-static-special-bait");
    write_law_rows(&root);
    let path = root.join(STATIC_REL);
    fs::create_dir_all(path.parent().expect("parent")).expect("parent");
    let outside = root.with_file_name("observe-static-special-secret");
    fs::write(&outside, "SECRET_CANARY").expect("outside canary");
    std::os::unix::fs::symlink(&outside, &path).expect("symlink");
    let before = metadata_snapshot(&root);
    let mut symlink_failures = Vec::new();
    check(&root, &mut symlink_failures);
    assert_eq!(before, metadata_snapshot(&root));
    assert_eq!(
        symlink_failures,
        vec![SUCCESSOR_CATALOG_UNAVAILABLE.to_string()]
    );
    assert!(!symlink_failures.join("\n").contains("SECRET_CANARY"));

    fs::remove_file(&path).expect("remove symlink");
    let name = CString::new(path.as_os_str().as_bytes()).expect("fifo name");
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert_eq!(
        command_inventory_failures(&root),
        vec![SUCCESSOR_CATALOG_UNAVAILABLE.to_string()]
    );
    fs::remove_file(outside).expect("outside cleanup");
    fs::remove_dir_all(root).expect("cleanup");
}
