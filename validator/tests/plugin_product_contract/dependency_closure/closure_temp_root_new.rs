struct ClosureTempRoot(std::path::PathBuf);

impl ClosureTempRoot {
    fn new() -> Self {
        static NEXT_CLOSURE_ROOT: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "hul-build-closure-{}-{}",
            std::process::id(),
            NEXT_CLOSURE_ROOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("src")).unwrap();
        std::fs::create_dir_all(path.join("target")).unwrap();
        std::fs::write(path.join("Cargo.toml"), b"[package]\nname='probe'\n").unwrap();
        std::fs::write(path.join("Cargo.lock"), b"version = 4\n").unwrap();
        std::fs::write(path.join("src/lib.rs"), b"pub fn probe() {}\n").unwrap();
        std::fs::write(path.join("runtime.json"), b"{\"runtime\":true}\n").unwrap();
        std::fs::write(path.join("verifier.rs"), b"pub fn verify() {}\n").unwrap();
        let dep_info = format!("target/probe: {}\n", path.join("src/lib.rs").display());
        std::fs::write(path.join("target/probe.d"), dep_info).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for ClosureTempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn build_policy(
    root: &std::path::Path,
) -> super::plugin_product::source_closure::BuildClosurePolicy {
    use super::plugin_product::source_closure::{
        BuildClosurePolicy, BuildInputKind, RequiredBuildInput,
    };
    BuildClosurePolicy::from_dep_info(
        root,
        &["target/probe.d".to_owned()],
        vec![
            RequiredBuildInput {
                path: "Cargo.lock".to_owned(),
                kind: BuildInputKind::CargoLock,
            },
            RequiredBuildInput {
                path: "Cargo.toml".to_owned(),
                kind: BuildInputKind::CargoManifest,
            },
            RequiredBuildInput {
                path: "runtime.json".to_owned(),
                kind: BuildInputKind::RuntimeAuthority,
            },
            RequiredBuildInput {
                path: "verifier.rs".to_owned(),
                kind: BuildInputKind::VerifierInput,
            },
        ],
    )
    .unwrap()
}

#[test]
fn build_closure_captures_dep_source_runtime_and_verifier_inputs_deterministically() {
    use super::plugin_product::source_closure::BuildClosureV1;
    let root = ClosureTempRoot::new();
    let policy = build_policy(root.path());
    let first = BuildClosureV1::capture(root.path(), &policy).unwrap();
    let second = BuildClosureV1::capture(root.path(), &policy).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.schema_version, "BuildClosure-v1");
    assert!(first.final_session_revalidated);
    assert_eq!(first.rows.len(), 6);
    first.verify(root.path(), &policy).unwrap();
}

#[test]
fn build_closure_rejects_unknown_missing_duplicate_and_alias_rows() {
    use super::plugin_product::source_closure::{
        BuildClosurePolicy, BuildClosureV1, BuildInputKind, ClosureError, RequiredBuildInput,
    };
    let root = ClosureTempRoot::new();
    let policy = build_policy(root.path());
    let live = BuildClosureV1::capture(root.path(), &policy).unwrap();

    let mut unknown = live.clone();
    unknown.rows[0].path = "unknown.rs".to_owned();
    assert_eq!(
        unknown.verify(root.path(), &policy),
        Err(ClosureError::UnknownRow)
    );
    let mut missing = live;
    missing.rows.pop();
    assert_eq!(
        missing.verify(root.path(), &policy),
        Err(ClosureError::UnknownRow)
    );
    assert_eq!(
        BuildClosurePolicy::new(vec![
            RequiredBuildInput {
                path: "src/lib.rs".to_owned(),
                kind: BuildInputKind::RustSource,
            },
            RequiredBuildInput {
                path: "SRC/LIB.RS".to_owned(),
                kind: BuildInputKind::RustSource,
            },
        ]),
        Err(ClosureError::DuplicateOrAlias)
    );
}

#[test]
fn representative_input_mutation_changes_anchor_and_stales_prior_closure() {
    use super::plugin_product::source_closure::{BuildClosureV1, ClosureError};
    let root = ClosureTempRoot::new();
    let policy = build_policy(root.path());
    let before = BuildClosureV1::capture(root.path(), &policy).unwrap();
    std::fs::write(root.path().join("runtime.json"), b"{\"runtime\":false}\n").unwrap();
    assert_eq!(
        before.verify(root.path(), &policy),
        Err(ClosureError::DigestMismatch)
    );
    let after = BuildClosureV1::capture(root.path(), &policy).unwrap();
    assert_ne!(after.aggregate_sha256, before.aggregate_sha256);
}

#[test]
fn final_session_drift_is_rejected_after_initial_read() {
    use super::plugin_product::source_closure::{BuildClosureV1, ClosureError};
    let root = ClosureTempRoot::new();
    let policy = build_policy(root.path());
    let result = BuildClosureV1::capture_with_observer(root.path(), &policy, |index, path| {
        if index == 0 {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
            file.write_all(b"# drift\n").unwrap();
        }
    });
    assert_eq!(result, Err(ClosureError::FinalSessionDrift));
}

#[cfg(unix)]
#[test]
fn build_closure_rejects_symlink_hardlink_and_special_inputs() {
    use super::plugin_product::source_closure::{
        BuildClosurePolicy, BuildClosureV1, BuildInputKind, ClosureError, RequiredBuildInput,
    };
    use std::os::unix::fs::symlink;
    let root = ClosureTempRoot::new();
    symlink("lib.rs", root.path().join("src/alias.rs")).unwrap();
    let alias_policy = BuildClosurePolicy::new(vec![RequiredBuildInput {
        path: "src/alias.rs".to_owned(),
        kind: BuildInputKind::RustSource,
    }])
    .unwrap();
    assert_eq!(
        BuildClosureV1::capture(root.path(), &alias_policy),
        Err(ClosureError::SpecialFile)
    );

    std::fs::hard_link(
        root.path().join("src/lib.rs"),
        root.path().join("src/hard.rs"),
    )
    .unwrap();
    let hard_policy = BuildClosurePolicy::new(vec![RequiredBuildInput {
        path: "src/lib.rs".to_owned(),
        kind: BuildInputKind::RustSource,
    }])
    .unwrap();
    assert_eq!(
        BuildClosureV1::capture(root.path(), &hard_policy),
        Err(ClosureError::SpecialFile)
    );

    let fifo = root.path().join("special.fifo");
    let path = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let fifo_policy = BuildClosurePolicy::new(vec![RequiredBuildInput {
        path: "special.fifo".to_owned(),
        kind: BuildInputKind::DynamicInput,
    }])
    .unwrap();
    assert_eq!(
        BuildClosureV1::capture(root.path(), &fifo_policy),
        Err(ClosureError::SpecialFile)
    );
}
