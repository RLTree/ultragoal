use super::digest::{file_identity, file_identity_regular, set_test_pauses};
use crate::context::{BuildRequest, LiveContext};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Repo {
    root: PathBuf,
}

impl Repo {
    fn new(label: &str, files: &[(&str, &[u8])]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-inventory-race-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        git(&root, &["init", "-q"]);
        git(
            &root,
            &["config", "user.email", "inventory@example.invalid"],
        );
        git(&root, &["config", "user.name", "Inventory Race Test"]);
        for (name, bytes) in files {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "fixture"]);
        Self { root }
    }
}

#[test]
fn pinned_parent_rejects_directory_swap_and_restore() {
    let original = vec![b'a'; 32 * 1024];
    let replacement = vec![b'b'; 32 * 1024];
    let repo = Repo::new(
        "directory-swap",
        &[
            ("collection/source.bin", original.as_slice()),
            ("replacement/source.bin", replacement.as_slice()),
        ],
    );
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let reads = context.begin_read_session().unwrap();
    let collection = repo.root.join("collection");
    let saved = repo.root.join("collection.saved");
    let replacement_dir = repo.root.join("replacement");
    reads.pin_directory(&collection).unwrap();
    fs::rename(&collection, &saved).unwrap();
    fs::rename(&replacement_dir, &collection).unwrap();
    let result = file_identity(&reads, &collection.join("source.bin"));
    fs::rename(&collection, &replacement_dir).unwrap();
    fs::rename(&saved, &collection).unwrap();
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("file identity changed")
    );
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("/usr/bin/git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn regular_file_swap_between_identity_and_open_is_rejected() {
    let original = vec![b'a'; 32 * 1024];
    let replacement = vec![b'b'; 32 * 1024];
    let repo = Repo::new(
        "swap",
        &[
            ("source.bin", original.as_slice()),
            ("replacement.bin", replacement.as_slice()),
        ],
    );
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let reads = context.begin_read_session().unwrap();
    let target = repo.root.join("source.bin");
    let target_for_swap = target.clone();
    let replacement_path = repo.root.join("replacement.bin");
    set_test_pauses(150, 0);
    let swap = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        fs::rename(&target_for_swap, target_for_swap.with_extension("original")).unwrap();
        fs::rename(replacement_path, target_for_swap).unwrap();
    });
    let result = file_identity(&reads, &target);
    swap.join().unwrap();
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("file identity changed")
    );
}

#[cfg(unix)]
#[test]
fn plugin_regular_identity_rejects_manifest_and_hook_symlink_swaps() {
    for (label, relative) in [
        ("plugin-manifest-symlink-swap", ".codex-plugin/plugin.json"),
        ("plugin-hook-symlink-swap", "hooks/hooks.json"),
    ] {
        let repo = Repo::new(label, &[(relative, b"{}\n")]);
        let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
        let reads = context.begin_read_session().unwrap();
        let target = repo.root.join(relative);
        let swap_target = target.clone();
        set_test_pauses(150, 0);
        let swap = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            fs::rename(&swap_target, swap_target.with_extension("original")).unwrap();
            std::os::unix::fs::symlink("/dev/null", &swap_target).unwrap();
        });
        let result = file_identity_regular(&reads, &target);
        swap.join().unwrap();
        assert!(result.is_err(), "{relative} symlink swap was accepted");
    }
}

#[test]
fn in_place_mutate_and_restore_during_hash_is_rejected() {
    let original = vec![b'a'; 64 * 1024];
    let mutated = vec![b'b'; 64 * 1024];
    let repo = Repo::new("mutate", &[("source.bin", original.as_slice())]);
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let reads = context.begin_read_session().unwrap();
    let target = repo.root.join("source.bin");
    let target_for_mutation = target.clone();
    set_test_pauses(0, 150);
    let mutation = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        fs::write(&target_for_mutation, &mutated).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        fs::write(&target_for_mutation, &original).unwrap();
    });
    let result = file_identity(&reads, &target);
    mutation.join().unwrap();
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("file identity changed")
    );
}
