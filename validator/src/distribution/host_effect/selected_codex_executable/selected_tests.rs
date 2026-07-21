use super::super::HostEffectLedgerError;
use super::{path_byte_length, resolve_from_path};
use crate::distribution::error::DistributionError;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

pub(crate) struct SelectedCodexExecutableTestFixture {
    root: PathBuf,
    path: PathBuf,
    selected: super::SelectedCodexExecutable,
}

impl SelectedCodexExecutableTestFixture {
    pub(crate) fn selected(&self) -> Result<super::SelectedCodexExecutable, HostEffectLedgerError> {
        self.selected.duplicate()
    }

    pub(crate) fn host_capability(
        &self,
        home: &std::path::Path,
        project: &std::path::Path,
        host_version: &str,
    ) -> Result<crate::distribution::HostCapabilityDeclaration, DistributionError> {
        crate::distribution::HostCapabilityDeclaration::isolated(
            home,
            project,
            host_version,
            Some(&self.path),
        )
    }

    pub(crate) fn replace_contents(&self, bytes: &[u8]) {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .expect("fixture executable replacement");
        std::io::Write::write_all(&mut file, bytes).expect("fixture executable bytes");
        file.sync_all().expect("fixture executable sync");
    }

    pub(crate) fn rename_and_replace(&self, bytes: &[u8]) {
        fs::rename(&self.path, self.root.join("held")).expect("fixture executable rename");
        fs::write(&self.path, bytes).expect("fixture executable replacement");
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o700))
            .expect("fixture executable mode");
    }

    pub(crate) fn replace_with_hard_link_from(&self, source: &Self) {
        fs::remove_file(&self.path).expect("fixture executable removal");
        fs::hard_link(&source.path, &self.path).expect("fixture executable hard link");
    }
}

impl Drop for SelectedCodexExecutableTestFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(crate) fn selected_test_fixture(
    label: &str,
    bytes: &[u8],
) -> SelectedCodexExecutableTestFixture {
    let root = std::env::temp_dir().join(format!(
        "harness-ultragoal-selected-executable-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).expect("fixture directory");
    let path = root.join("codex");
    fs::write(&path, bytes).expect("fixture executable bytes");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("fixture executable mode");
    let selected = resolve_from_path([root.clone()]).expect("fixture executable selection");
    SelectedCodexExecutableTestFixture {
        root,
        path,
        selected,
    }
}

#[test]
fn path_limit_counts_operating_system_string_bytes() {
    let path = OsString::from("é".repeat(64));
    assert_eq!(path_byte_length(&path), 128);
}

#[test]
fn path_resolution_accepts_a_user_local_executable_without_a_private_allowlist() {
    let root = test_root("path-resolution");
    fs::create_dir_all(&root).expect("directory");
    let target = root.join("codex-target");
    fs::write(&target, b"codex").expect("executable bytes");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).expect("executable mode");
    let executable = root.join("codex");
    symlink(&target, &executable).expect("codex symlink");

    assert!(resolve_from_path([root.clone()]).is_ok());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn pinned_selection_rejects_in_place_replacement_after_path_resolution() {
    let root = test_root("path-resolution-replacement");
    fs::create_dir_all(&root).expect("directory");
    let executable = root.join("codex");
    fs::write(&executable, b"first").expect("executable bytes");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("executable mode");

    let pinned = resolve_from_path([root.clone()]).expect("resolved");
    fs::write(&executable, b"replacement").expect("replacement bytes");

    assert!(pinned.revalidate().is_err());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn pinned_selection_keeps_original_target_after_symlink_replacement() {
    let root = test_root("path-resolution-symlink-replacement");
    fs::create_dir_all(&root).expect("directory");
    let first = root.join("codex-first");
    let second = root.join("codex-second");
    fs::write(&first, b"first").expect("first executable bytes");
    fs::write(&second, b"second").expect("second executable bytes");
    for path in [&first, &second] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("executable mode");
    }
    let executable = root.join("codex");
    symlink(&first, &executable).expect("codex symlink");

    let pinned = resolve_from_path([root.clone()]).expect("resolved");
    let original_binding = pinned
        .binding_sha256()
        .expect("selected executable binding");
    fs::remove_file(&executable).expect("remove symlink");
    symlink(&second, &executable).expect("replacement symlink");

    assert!(pinned.revalidate().is_ok());
    assert_eq!(
        pinned
            .binding_sha256()
            .expect("selected executable binding"),
        original_binding
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn path_resolution_skips_relative_and_non_executable_entries() {
    let root = test_root("path-resolution-negative");
    fs::create_dir_all(&root).expect("directory");
    let executable = root.join("codex");
    fs::write(&executable, b"codex").expect("executable bytes");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o644))
        .expect("non-executable mode");

    assert!(resolve_from_path([PathBuf::from("relative"), root.clone()]).is_err());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn path_resolution_rejects_a_hard_linked_executable() {
    let root = test_root("path-resolution-hard-link");
    fs::create_dir_all(&root).expect("directory");
    let target = root.join("codex-target");
    fs::write(&target, b"codex").expect("executable bytes");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).expect("executable mode");
    fs::hard_link(&target, root.join("codex")).expect("hard link");

    assert!(resolve_from_path([root.clone()]).is_err());
    fs::remove_dir_all(root).expect("cleanup");
}

fn test_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "harness-ultragoal-codex-executable-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ))
}
