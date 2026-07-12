use super::artifact::{self, ReadStage};
use serde_json::json;
#[cfg(not(unix))]
use std::path::Path;
use std::path::PathBuf;

#[cfg(unix)]
mod corrective;

#[cfg(unix)]
#[test]
fn bound_reader_rejects_in_place_mutation_after_read() {
    let (root, path) = anchor("anchor-read-race", br#"{"status":"pass","pad":"aaaa"}"#);
    let result = artifact::read_with_hook(&root, "fixtures/anchor.json", |stage| {
        if stage == ReadStage::AfterRead {
            std::fs::write(&path, br#"{"status":"pass","pad":"bbbb"}"#).expect("mutate");
        }
    });
    assert_error(result, "review_round_anchor_changed_during_read");
    std::fs::remove_dir_all(root).expect("cleanup race");
}

#[cfg(unix)]
#[test]
fn bound_reader_rejects_path_swap_after_read() {
    let (root, path) = anchor("anchor-read-swap", br#"{"status":"pass","pad":"aaaa"}"#);
    let result = artifact::read_with_hook(&root, "fixtures/anchor.json", |stage| {
        if stage == ReadStage::AfterRead {
            std::fs::rename(&path, path.with_extension("old")).expect("move original");
            std::fs::write(&path, br#"{"status":"pass","pad":"bbbb"}"#).expect("replacement");
        }
    });
    assert_error(result, "review_round_anchor_changed_during_read");
    std::fs::remove_dir_all(root).expect("cleanup swap");
}

#[cfg(unix)]
#[test]
fn bound_reader_rejects_ancestor_substitution_before_open() {
    use std::os::unix::fs::symlink;

    let (root, _) = anchor("anchor-preopen-ancestor", br#"{"status":"trusted"}"#);
    let outside = root.with_extension("outside");
    let held = root.join("held");
    std::fs::create_dir_all(&outside).expect("outside");
    std::fs::write(outside.join("anchor.json"), br#"{"status":"escaped"}"#)
        .expect("escaped anchor");
    let result = artifact::read_with_hook(&root, "fixtures/anchor.json", |stage| {
        if stage == ReadStage::BeforeAncestorOpen(0) {
            std::fs::rename(root.join("fixtures"), &held).expect("hold trusted ancestor");
            symlink(&outside, root.join("fixtures")).expect("substitute ancestor");
        }
    });
    assert_error(result, "review_round_anchor_path_invalid");
    std::fs::remove_file(root.join("fixtures")).expect("remove link");
    std::fs::rename(&held, root.join("fixtures")).expect("restore ancestor");
    std::fs::remove_dir_all(&root).expect("cleanup root");
    std::fs::remove_dir_all(outside).expect("cleanup outside");
}

#[cfg(unix)]
#[test]
fn bound_reader_rejects_root_substitution_before_open() {
    use std::os::unix::fs::symlink;

    let (root, _) = anchor("anchor-preopen-root", br#"{"status":"trusted"}"#);
    let held = root.with_extension("held");
    let outside = root.with_extension("outside");
    std::fs::create_dir_all(outside.join("fixtures")).expect("outside");
    std::fs::write(
        outside.join("fixtures/anchor.json"),
        br#"{"status":"escaped"}"#,
    )
    .expect("escaped anchor");
    let result = artifact::read_with_hook(&root, "fixtures/anchor.json", |stage| {
        if stage == ReadStage::BeforeRootOpen {
            std::fs::rename(&root, &held).expect("hold trusted root");
            symlink(&outside, &root).expect("substitute root");
        }
    });
    assert_error(result, "review_round_anchor_path_invalid");
    std::fs::remove_file(&root).expect("remove root link");
    std::fs::rename(&held, &root).expect("restore root");
    std::fs::remove_dir_all(&root).expect("cleanup root");
    std::fs::remove_dir_all(outside).expect("cleanup outside");
}

#[cfg(unix)]
#[test]
fn bound_reader_revalidates_opened_ancestor_identity() {
    use std::os::unix::fs::symlink;

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("anchor-parent-swap");
    let path = root.join("fixtures/nested/anchor.json");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("parent");
    std::fs::write(&path, br#"{"status":"trusted"}"#).expect("trusted");
    let held = root.join("held");
    let outside = root.with_extension("outside");
    std::fs::create_dir_all(outside.join("nested")).expect("outside");
    let result = artifact::read_with_hook(&root, "fixtures/nested/anchor.json", |stage| {
        if stage == ReadStage::AncestorOpened(0) {
            std::fs::rename(root.join("fixtures"), &held).expect("hold ancestor");
            symlink(&outside, root.join("fixtures")).expect("replace ancestor");
        }
    });
    assert_error(result, "review_round_anchor_changed_during_read");
    std::fs::remove_file(root.join("fixtures")).expect("remove link");
    std::fs::rename(&held, root.join("fixtures")).expect("restore ancestor");
    std::fs::remove_dir_all(&root).expect("cleanup root");
    std::fs::remove_dir_all(outside).expect("cleanup outside");
}

#[cfg(unix)]
#[test]
fn bound_reader_rejects_symlink_hardlink_and_special_file_without_echo() {
    use std::os::unix::fs::symlink;

    for attack in ["leaf-symlink", "hardlink", "special"] {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(attack);
        std::fs::create_dir_all(root.join("fixtures")).expect("fixtures");
        let path = root.join("fixtures/SECRET_CANARY.json");
        let attacker = root.join("attacker.json");
        std::fs::write(&attacker, br#"{"status":"escaped"}"#).expect("attacker");
        match attack {
            "leaf-symlink" => symlink(&attacker, &path).expect("symlink"),
            "hardlink" => std::fs::hard_link(&attacker, &path).expect("hardlink"),
            "special" => std::fs::create_dir(&path).expect("directory"),
            _ => unreachable!(),
        }
        let result = artifact::read(&root, "fixtures/SECRET_CANARY.json");
        let expected = if attack == "leaf-symlink" {
            "review_round_anchor_path_invalid"
        } else {
            "review_round_anchor_not_regular"
        };
        assert_error(result, expected);
        assert!(!expected.contains("SECRET_CANARY"));
        std::fs::remove_dir_all(root).expect("cleanup attack");
    }
}

#[cfg(unix)]
#[test]
fn bound_reader_rejects_duplicate_keys_and_trailing_json() {
    for (label, bytes) in [
        ("duplicate-top", br#"{"a":1,"a":2}"#.as_slice()),
        ("duplicate-nested", br#"{"a":{"b":1,"b":2}}"#.as_slice()),
        ("trailing", br#"{"a":1}{"b":2}"#.as_slice()),
    ] {
        let (root, _) = anchor(label, bytes);
        assert_error(
            artifact::read(&root, "fixtures/anchor.json"),
            "review_round_anchor_malformed",
        );
        std::fs::remove_dir_all(root).expect("cleanup malformed");
    }
}

#[cfg(unix)]
#[test]
fn bound_reader_reports_missing_anchor_without_echo() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("anchor-missing");
    std::fs::create_dir_all(root.join("fixtures")).expect("fixtures");
    assert_error(
        artifact::read(&root, "fixtures/SECRET_CANARY.json"),
        "review_round_anchor_unavailable",
    );
    std::fs::remove_dir_all(root).expect("cleanup missing");
}

#[cfg(unix)]
#[test]
fn bound_reader_rejects_oversized_anchor_before_parsing() {
    let bytes = vec![b' '; 4 * 1024 * 1024 + 1];
    let (root, _) = anchor("anchor-too-large", &bytes);
    assert_error(
        artifact::read(&root, "fixtures/anchor.json"),
        "review_round_anchor_too_large",
    );
    std::fs::remove_dir_all(root).expect("cleanup oversized");
}

#[cfg(unix)]
#[test]
fn bound_reader_uses_one_snapshot_and_performs_no_write() {
    let bytes = serde_json::to_vec(&json!({"status":"pass"})).expect("json");
    let (root, path) = anchor("anchor-read-only", &bytes);
    let before = crate::digest::file(&path).expect("before");
    let artifact = artifact::read(&root, "fixtures/anchor.json").expect("read");
    let after = crate::digest::file(&path).expect("after");
    assert_eq!(before, after);
    assert_eq!(artifact.digest, crate::digest::bytes(&bytes));
    assert_eq!(artifact.value["status"], "pass");
    assert_eq!(std::fs::read_dir(root.join("fixtures")).unwrap().count(), 1);
    std::fs::remove_dir_all(root).expect("cleanup read only");
}

fn anchor(label: &str, bytes: &[u8]) -> (PathBuf, PathBuf) {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    let path = root.join("fixtures/anchor.json");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("fixtures");
    std::fs::write(&path, bytes).expect("anchor");
    (root, path)
}

fn assert_error(result: Result<artifact::JsonArtifact, String>, expected: &str) {
    let error = match result {
        Ok(_) => panic!("untrusted anchor was accepted"),
        Err(error) => error,
    };
    assert_eq!(error, expected);
}

#[cfg(not(unix))]
#[test]
fn bound_reader_fails_closed_without_descriptor_walk_support() {
    assert_error(
        artifact::read(Path::new("."), "fixtures/anchor.json"),
        "review_round_anchor_path_invalid",
    );
}
