#[cfg(target_os = "macos")]
#[test]
fn supported_codex_observer_denies_child_writes_and_accepts_only_closed_outputs() {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(1);
    let parent = crate::distribution::canonical_temporary_parent().unwrap();
    let root = parent.join(format!(
        "supported-codex-read-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let home = root.join("home");
    let cwd = root.join("worktree");
    std::fs::create_dir_all(home.join(".codex")).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();
    let executable = root.join("codex");
    let script = br##"#!/bin/sh
exec 2>&-
if : > "$HOME/oracle-write-canary"; then
  exit 92
fi
case "$*" in
  "plugin add --help")
    printf 'Usage: codex plugin add [OPTIONS] <PLUGIN[@MARKETPLACE]>\n\nArguments:\n  <PLUGIN[@MARKETPLACE]>\n'
    ;;
  "plugin list --json")
    printf '{"installed":[]}'
    ;;
  "plugin marketplace list --json")
    printf '{"marketplaces":[]}'
    ;;
  *)
    exit 93
    ;;
esac
"##;
    std::fs::write(&executable, script).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let expected_sha256 = format!("sha256:{:x}", Sha256::digest(script));

    let observation = SelectedCodexExecutable::observe_supported_read_only(
        &executable,
        &expected_sha256,
        &home,
        &cwd,
    )
    .unwrap();

    let (path, content_sha256, binding_sha256, help, plugins, marketplaces) = observation;
    assert_eq!(path, executable.canonicalize().unwrap());
    assert_eq!(content_sha256, expected_sha256);
    assert!(binding_sha256.starts_with("sha256:"));
    assert_eq!(binding_sha256.len(), 71);
    assert!(
        String::from_utf8(help)
            .unwrap()
            .contains("<PLUGIN[@MARKETPLACE]>")
    );
    assert_eq!(plugins, br#"{"installed":[]}"#);
    assert_eq!(marketplaces, br#"{"marketplaces":[]}"#);
    assert!(!home.join("oracle-write-canary").exists());
    let sandbox_source = include_str!("../selected_codex_executable/execution/darwin_sandbox.rs");
    for required in [
        "(deny file-write*)",
        "(deny file-clone file-link)",
        "(deny network*)",
        "(deny process-fork (with send-signal SIGKILL))",
    ] {
        assert!(sandbox_source.contains(required));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn supported_codex_observer_rejects_unexpected_stderr() {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(1);
    let parent = crate::distribution::canonical_temporary_parent().unwrap();
    let root = parent.join(format!(
        "supported-codex-stderr-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let home = root.join("home");
    let cwd = root.join("worktree");
    std::fs::create_dir_all(home.join(".codex")).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();
    let executable = root.join("codex");
    let script = b"#!/bin/sh\nprintf 'unexpected diagnostic\\n' >&2\nprintf 'Usage: codex plugin add [OPTIONS] <PLUGIN[@MARKETPLACE]>\\nArguments:\\n  <PLUGIN[@MARKETPLACE]>\\n'\n";
    std::fs::write(&executable, script).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let expected_sha256 = format!("sha256:{:x}", Sha256::digest(script));

    assert!(
        SelectedCodexExecutable::observe_supported_read_only(
            &executable,
            &expected_sha256,
            &home,
            &cwd,
        )
        .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}
