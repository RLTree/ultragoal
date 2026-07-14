use super::*;

#[test]
pub(crate) fn public_binary_refuses_acceptance_framing_and_host_substitution_without_effect() {
    let fixture = Fixture::new("refusals");
    let (plan_sha256, plan_bytes) = fixture.plan();
    let bad_digest = format!("sha256:{}", "0".repeat(64));

    let before_root = snapshot(&fixture.root);
    let before_home = snapshot(&fixture.home);
    let mismatch = fixture.apply(&bad_digest);
    assert_diagnostic(
        &mismatch,
        3,
        "successor_runtime_authority_required",
        &fixture,
    );
    assert_eq!(snapshot(&fixture.root), before_root);
    assert_eq!(snapshot(&fixture.home), before_home);
    assert!(!fixture.root.join("AGENTS.md").exists());

    OpenOptions::new()
        .append(true)
        .open(fixture.root.join("validation_artifacts/fit-plan.json"))
        .unwrap()
        .write_all(b"\n")
        .unwrap();
    let before_root = snapshot(&fixture.root);
    let before_home = snapshot(&fixture.home);
    let alternate_framing = fixture.apply(&plan_sha256);
    assert_diagnostic(
        &alternate_framing,
        2,
        "successor_runtime_unexpected_arguments",
        &fixture,
    );
    assert_eq!(snapshot(&fixture.root), before_root);
    assert_eq!(snapshot(&fixture.home), before_home);

    fs::write(
        fixture.root.join("validation_artifacts/fit-plan.json"),
        plan_bytes,
    )
    .unwrap();
    fs::remove_dir(&fixture.pending).unwrap();
    let substituted = fixture.container.join("substituted-pending");
    fs::create_dir(&substituted).unwrap();
    fs::set_permissions(&substituted, fs::Permissions::from_mode(0o700)).unwrap();
    symlink(&substituted, &fixture.pending).unwrap();
    let before_root = snapshot(&fixture.root);
    let before_home = snapshot(&fixture.home);
    let refusal = fixture.apply(&plan_sha256);
    assert_diagnostic(
        &refusal,
        3,
        "successor_runtime_authority_required",
        &fixture,
    );
    assert_eq!(snapshot(&fixture.root), before_root);
    assert_eq!(snapshot(&fixture.home), before_home);
    assert!(!fixture.root.join("AGENTS.md").exists());
}

#[test]
pub(crate) fn public_binary_rejects_a_stale_plan_before_opening_host_authority() {
    let fixture = Fixture::new("stale-plan");
    let (plan_sha256, _) = fixture.plan();
    fs::write(
        fixture.root.join("private-user-change.txt"),
        b"preserve me\n",
    )
    .unwrap();
    let before_root = snapshot(&fixture.root);
    let before_home = snapshot(&fixture.home);
    let before_status = status(&fixture.root);
    let stale = fixture.apply(&plan_sha256);
    assert_diagnostic(&stale, 1, "successor_runtime_stale_context", &fixture);
    assert_eq!(snapshot(&fixture.root), before_root);
    assert_eq!(snapshot(&fixture.home), before_home);
    assert_eq!(status(&fixture.root), before_status);
    assert!(!fixture.root.join("AGENTS.md").exists());
}

pub(crate) fn assert_diagnostic(output: &Output, code: i32, id: &str, fixture: &Fixture) {
    assert_eq!(output.status.code(), Some(code), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(value["diagnostic_id"], id);
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(!text.contains(&*fixture.root.to_string_lossy()));
    assert!(!text.contains(&*fixture.home.to_string_lossy()));
}

pub(crate) fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub(crate) fn copy_authority_inputs(live: &Path, root: &Path) {
    let source = live.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    let target = root.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    fs::create_dir_all(&target).unwrap();
    let mut files = fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    for path in files {
        fs::copy(&path, target.join(path.file_name().unwrap())).unwrap();
    }
    for name in ["FINAL-HANDOFF-MANIFEST.sha256", "README.md"] {
        fs::copy(
            source.parent().unwrap().join(name),
            target.parent().unwrap().join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(root.join("migration")).unwrap();
    for name in ["authority-routes.json", "generated-surface-authority.json"] {
        fs::copy(
            live.join("migration").join(name),
            root.join("migration").join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    fs::write(
        root.join(".codex-plugin/plugin.json"),
        br#"{"name":"harness-ultragoal","version":"0.0.0-test"}"#,
    )
    .unwrap();
    fs::write(root.join(".gitignore"), b"validation_artifacts/\n").unwrap();
}

pub(crate) fn git(root: &Path, args: &[&str]) -> Output {
    let output = Command::new("/usr/bin/git")
        .args(args)
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    output
}

pub(crate) fn status(root: &Path) -> Vec<u8> {
    git(
        root,
        &[
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ],
    )
    .stdout
}

pub(crate) fn snapshot(root: &Path) -> Vec<SnapshotRow> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<SnapshotRow>) {
        let metadata = fs::symlink_metadata(path).unwrap();
        let kind = if metadata.is_file() {
            "file"
        } else if metadata.is_dir() {
            "directory"
        } else if metadata.file_type().is_symlink() {
            "symlink"
        } else {
            "special"
        };
        let content = if metadata.is_file() {
            fs::read(path).unwrap()
        } else if metadata.file_type().is_symlink() {
            fs::read_link(path).unwrap().as_os_str().as_bytes().to_vec()
        } else {
            Vec::new()
        };
        rows.push(SnapshotRow {
            path: path.strip_prefix(root).unwrap().to_path_buf(),
            kind,
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode: metadata.mode(),
            size: metadata.size(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
            content_sha256: format!("sha256:{:x}", Sha256::digest(content)),
        });
        if metadata.is_dir() {
            let mut entries = fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            entries.sort();
            for entry in entries {
                visit(root, &entry, rows);
            }
        }
    }
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}

pub(crate) fn pending_entries(pending: &Path) -> Vec<String> {
    let mut names = fs::read_dir(pending)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    names
}
