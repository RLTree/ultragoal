use super::{PackageCapture, PackageEntryKind, PackageSnapshot};
use crate::context::{BuildRequest, LiveContext};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Repo {
    root: PathBuf,
}

impl Repo {
    fn new(label: &str) -> Self {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
        fs::create_dir_all(root.join(".codex-plugin")).expect("plugin manifest dir");
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            br#"{"name":"snapshot-test","version":"0.0.0","skills":"./skills/"}"#,
        )
        .expect("plugin manifest");
        fs::create_dir_all(root.join("schemas")).expect("schema dir");
        fs::write(root.join("schemas/catalog.json"), "{}\n").expect("catalog");
        fs::write(root.join("resource.txt"), "trusted\n").expect("resource");
        write_manifest(&root, json!(["plugin-manifest-draft.json", "resource.txt"]));
        let output = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&root)
            .output()
            .expect("git init");
        assert!(output.status.success());
        Self { root }
    }

    fn context(&self) -> LiveContext {
        LiveContext::build(
            BuildRequest::new(&self.root)
                .expect_worktree_root(&self.root)
                .expect_repository_root(&self.root),
        )
        .expect("live context")
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_manifest(root: &Path, resources: serde_json::Value) {
    fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({
            "name": "snapshot-test",
            "version": "0.0.0",
            "status": "test",
            "purpose": "test",
            "skills": [],
            "agents": [],
            "schemas": [],
            "fixtures": [],
            "authorable_templates": [],
            "generated_examples": [],
            "resources": resources,
            "schema_catalog": "schemas/catalog.json",
            "optional_connectors": [],
            "non_goals": []
        }))
        .expect("manifest json"),
    )
    .expect("manifest");
}

fn status(root: &Path) -> Vec<u8> {
    Command::new("git")
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ])
        .current_dir(root)
        .output()
        .expect("git status")
        .stdout
}

#[test]
fn snapshot_is_context_bound_deterministic_and_digest_compatible() {
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
    let expected = super::super::package_digest(&repo.root).expect("legacy digest");

    let finished = PackageCapture::begin(&context)
        .expect("snapshot capture")
        .finish()
        .expect("snapshot finish");
    assert_eq!(finished.context_id(), context.context_id());
    assert_eq!(finished.package_digest(), expected);
    assert_eq!(
        finished.manifest_bytes(),
        finished.bytes("plugin-manifest-draft.json").unwrap()
    );
    assert!(finished.manifest().is_object());
    assert_eq!(finished.listed_paths().len(), 3);
    assert_eq!(finished.packaged_paths(), [".codex-plugin/plugin.json"]);
    assert!(finished.dependency_paths().len() >= finished.listed_paths().len());
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
    assert_eq!(second.package_digest(), finished.package_digest());
    assert_eq!(status(&repo.root), before);
    assert!(!repo.root.join("validation_artifacts").exists());
}

#[test]
fn snapshot_tree_preserves_symlink_special_and_hardlink_facts() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    let repo = Repo::new("package-snapshot-entry-kinds");
    fs::write(repo.root.join("ordinary.txt"), "ordinary").expect("ordinary");
    fs::hard_link(
        repo.root.join("ordinary.txt"),
        repo.root.join("hardlink.txt"),
    )
    .expect("hardlink");
    symlink("ordinary.txt", repo.root.join("symlink.txt")).expect("symlink");
    let fifo = repo.root.join("special.fifo");
    let name = CString::new(fifo.as_os_str().as_bytes()).expect("fifo name");
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let context = repo.context();

    let snapshot = PackageCapture::begin(&context)
        .expect("snapshot capture")
        .finish()
        .expect("snapshot finish");
    assert_eq!(
        snapshot.tree().get("ordinary.txt"),
        Some(&PackageEntryKind::Regular { single_link: false })
    );
    assert_eq!(
        snapshot.tree().get("hardlink.txt"),
        Some(&PackageEntryKind::Regular { single_link: false })
    );
    assert_eq!(
        snapshot.tree().get("symlink.txt"),
        Some(&PackageEntryKind::Symlink)
    );
    assert_eq!(
        snapshot.tree().get("special.fifo"),
        Some(&PackageEntryKind::Special)
    );
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
    assert_eq!(error, "package snapshot manifest collection is missing");
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

    let unix_capture = include_str!("capture.rs");
    let platform_surface = include_str!("mod.rs");
    assert!(!exposes_snapshot(unix_capture));
    assert!(!exposes_snapshot(platform_surface));
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
        json!([
            "plugin-manifest-draft.json",
            "resource.txt",
            "skills/target/SKILL.md"
        ]),
    );
    let context = repo.context();

    let snapshot = PackageCapture::begin(&context)
        .expect("snapshot capture")
        .finish()
        .expect("snapshot finish");

    assert!(
        snapshot
            .listed_paths()
            .iter()
            .any(|path| path == "skills/target/SKILL.md")
    );
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
