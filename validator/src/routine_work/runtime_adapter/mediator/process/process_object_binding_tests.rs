#[cfg(target_os = "macos")]
use super::super::filesystem::{OutputConfinement, PinnedExecutable, ReadConfinement, RootAnchor};
#[cfg(target_os = "macos")]
use super::super::outcome::RoutineCancellation;
#[cfg(target_os = "macos")]
use super::{execute, set_test_process_post_spawn_hook, set_test_process_pre_spawn_hook};
#[cfg(target_os = "macos")]
use crate::routine_work::{
    RustSourceFrameInput, encode_rust_source_syntax_frame, trusted_rust_source_execution_observed,
};
#[cfg(target_os = "macos")]
use sha2::{Digest, Sha256};
#[cfg(target_os = "macos")]
use std::collections::BTreeMap;
#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::os::unix::fs::PermissionsExt;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(target_os = "macos")]
static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "macos")]
#[test]
fn named_path_swap_during_spawn_cannot_authenticate_observation() {
    let root = unique_root("object-bound-spawn");
    let workspace = root.join("workspace");
    let directory = root.join("bin");
    let held_directory = root.join("bin-held");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&directory).unwrap();
    let named = directory.join("runner");
    fs::copy("/bin/sh", &named).unwrap();
    fs::set_permissions(&named, fs::Permissions::from_mode(0o555)).unwrap();
    let root_anchor = RootAnchor::open(&workspace).unwrap();
    let outputs = OutputConfinement::prepare(&root_anchor, &[], 1024).unwrap();
    let reads = ReadConfinement {
        sources: Vec::new(),
    };
    let program = PinnedExecutable::open_unbound(&named).unwrap();
    let environment = BTreeMap::from([(
        crate::routine_work::CHILD_MODE_ENV.to_owned(),
        crate::routine_work::CHILD_MODE_VALUE.to_owned(),
    )]);

    let before_held_directory = held_directory.clone();
    let before_directory = directory.clone();
    let before_name = named.clone();
    set_test_process_pre_spawn_hook(move || {
        fs::rename(&before_directory, &before_held_directory).unwrap();
        fs::create_dir(&before_directory).unwrap();
        fs::copy("/usr/bin/true", &before_name).unwrap();
        fs::set_permissions(&before_name, fs::Permissions::from_mode(0o555)).unwrap();
    });
    let after_directory = directory.clone();
    let after_held_directory = held_directory.clone();
    set_test_process_post_spawn_hook(move || {
        fs::remove_dir_all(&after_directory).unwrap();
        fs::rename(&after_held_directory, &after_directory).unwrap();
    });

    let observation = execute(
        &program,
        &root_anchor,
        &outputs,
        &reads,
        &[
            "sh".to_owned(),
            "-c".to_owned(),
            "printf retained-object".to_owned(),
        ],
        &environment,
        Vec::new(),
        Duration::from_secs(2),
        1024,
        &RoutineCancellation::new(),
        || Ok(()),
    )
    .unwrap();
    let source = b"pub fn value() -> u8 { 1 }\n";
    let source_digest = format!("sha256:{:x}", Sha256::digest(source));
    let frame = encode_rust_source_syntax_frame(&[RustSourceFrameInput::new(
        "src/lib.rs",
        &source_digest,
        source.len() as u64,
        source,
    )])
    .unwrap();
    assert_ne!(
        observation.stdout, b"retained-object",
        "termination={:?}",
        observation.termination
    );
    let exit_code = match observation.termination {
        super::ProcessTermination::Exited(code) => code,
        _ => -1,
    };
    assert!(
        !trusted_rust_source_execution_observed(
            Some(&frame),
            exit_code,
            &observation.stdout,
            observation.stderr_sha256 == format!("sha256:{:x}", Sha256::digest([])),
        ),
        "the substituted process observation must not authenticate a canonical routine success"
    );
    assert!(named.exists());
    assert!(!held_directory.exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "macos")]
fn unique_root(label: &str) -> PathBuf {
    let parent = std::env::var_os("CODEX_WORKTREE_TMP")
        .map(PathBuf::from)
        .expect("managed worktree tmp is required");
    parent.join(format!(
        "hul-routine-process-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ))
}
