use super::{PackageCapture, PackageEntryKind, PackageSnapshot, Repo, status, write_manifest};
use std::fs;

#[test]
fn snapshot_is_context_bound_and_deterministic() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<PackageSnapshot>();
    let repo = Repo::new("package-snapshot-deterministic");
    fs::create_dir_all(repo.root.join("validator/src")).expect("rust source dir");
    fs::write(
        repo.root.join("validator/src/unlisted.rs"),
        "fn unlisted() {}\n",
    )
    .expect("rust source");
    let context = repo.context();
    let before = status(&repo.root);
    let finished = PackageCapture::begin(&context)
        .expect("snapshot capture")
        .finish()
        .expect("snapshot finish");
    assert_eq!(finished.context_id(), context.context_id());
    assert_eq!(finished.manifest().name(), "snapshot-test");
    assert_eq!(finished.packaged_paths(), [".codex-plugin/plugin.json"]);
    assert_eq!(finished.unix_mode(".codex-plugin/plugin.json"), Some(0o644));
    assert_eq!(
        finished.bytes("validator/src/unlisted.rs"),
        Some("fn unlisted() {}\n".as_bytes())
    );

    let second = PackageCapture::begin(&context)
        .expect("repeat capture")
        .finish()
        .expect("repeat finish");
    assert_eq!(second.snapshot_id(), finished.snapshot_id());
    assert_eq!(status(&repo.root), before);
    assert!(!repo.root.join("validation_artifacts").exists());
}

#[test]
fn listed_hardlink_is_rejected_without_attacker_echo() {
    let repo = Repo::new("package-snapshot-listed-hardlink");
    fs::hard_link(
        repo.root.join("resource.txt"),
        repo.root.join("resource-alias.txt"),
    )
    .expect("hardlink");
    let context = repo.context();
    let error = match PackageCapture::begin(&context) {
        Ok(_) => panic!("hardlink was accepted"),
        Err(error) => error,
    };
    assert_eq!(error, "package snapshot file is unavailable");
    assert!(!error.contains("resource.txt"));
}

#[test]
fn invalid_or_missing_manifest_collections_fail_closed_without_echo() {
    let repo = Repo::new("package-snapshot-invalid-manifest");
    fs::write(
        repo.root.join("plugin-manifest-draft.json"),
        br#"{"resources":["/Users/attacker/canary"]}"#,
    )
    .expect("bad manifest");
    let context = repo.context();
    let error = match PackageCapture::begin(&context) {
        Ok(_) => panic!("invalid manifest was accepted"),
        Err(error) => error,
    };
    assert_eq!(error, "package manifest has an invalid or unknown field");
    assert!(!error.contains("attacker"));
}

#[test]
fn mutation_and_mutate_restore_both_block_finalization() {
    let repo = Repo::new("package-snapshot-final-mutation");
    let context = repo.context();
    let original = fs::read(repo.root.join("resource.txt")).expect("original resource");
    let capture = PackageCapture::begin(&context).expect("snapshot capture");
    fs::write(repo.root.join("resource.txt"), "mutated\n").expect("mutate");
    fs::write(repo.root.join("resource.txt"), &original).expect("restore");
    let error = capture.finish().expect_err("mutate restore rejected");
    assert_eq!(error, "package snapshot changed before finalization");

    let context = repo.context();
    let capture = PackageCapture::begin(&context).expect("second capture");
    fs::write(repo.root.join("resource.txt"), "persistent\n").expect("mutate");
    let error = capture.finish().expect_err("persistent mutation rejected");
    assert_eq!(error, "package snapshot changed before finalization");
}

#[test]
fn package_capture_has_no_pre_finalization_snapshot_accessor_regression() {
    fn exposes_snapshot(source: &str) -> bool {
        source.lines().any(|line| line.contains("fn snapshot("))
    }
    assert!(!exposes_snapshot(include_str!("../../capture/mod.rs")));
    assert!(!exposes_snapshot(include_str!("../../mod.rs")));
}

#[test]
fn synthetic_case_colliding_package_member_is_rejected() {
    let roots = vec!["skills/prove".to_string()];
    assert_eq!(
        super::super::capture::supported_skill_root("skills/prove/SKILL.md", &roots)
            .expect("exact member"),
        Some("skills/prove")
    );
    assert!(
        super::super::capture::supported_skill_root("skills/PROVE/SKILL.md", &roots).is_err(),
        "case-colliding member was assigned canonical package authority"
    );
}

#[test]
fn nested_target_package_resource_is_captured_after_finalization_regression() {
    let repo = Repo::new("package-snapshot-nested-target");
    fs::create_dir_all(repo.root.join("skills/target")).expect("nested target directory");
    fs::write(
        repo.root.join("skills/target/SKILL.md"),
        "legitimate package resource\n",
    )
    .expect("nested target resource");
    write_manifest(
        &repo.root,
        &[
            "plugin-manifest-draft.json",
            "resource.txt",
            "skills/target/SKILL.md",
        ],
    );
    let context = repo.context();
    let snapshot = PackageCapture::begin(&context)
        .expect("snapshot capture")
        .finish()
        .expect("snapshot finish");
    assert_eq!(
        snapshot.bytes("skills/target/SKILL.md"),
        Some("legitimate package resource\n".as_bytes())
    );
    assert_eq!(
        snapshot.tree().get("skills/target"),
        Some(&PackageEntryKind::Directory)
    );
    assert_eq!(
        snapshot.tree().get("skills/target/SKILL.md"),
        Some(&PackageEntryKind::Regular { single_link: true })
    );
}
