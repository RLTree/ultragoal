#[cfg(target_os = "macos")]
use super::super::filesystem::{OutputConfinement, PinnedExecutable, ReadConfinement, RootAnchor};
#[cfg(target_os = "macos")]
use super::super::outcome::RoutineCancellation;
#[cfg(target_os = "macos")]
use super::process_termination_tests::observe_prepared;
#[cfg(target_os = "macos")]
use super::{
    set_test_loaded_object_hook, set_test_process_post_spawn_hook, set_test_process_pre_spawn_hook,
};
#[cfg(target_os = "macos")]
use crate::routine_work::{
    RustSourceFrameInput, RustSourceSyntaxOutcome, encode_rust_source_syntax_frame,
    evaluate_rust_source_syntax_frame, rust_source_syntax_observation_json,
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
    let (frame, canonical) = canonical_observation();
    let sentinel = root.join("replacement-executed");
    let environment = BTreeMap::from([(
        crate::routine_work::CHILD_MODE_ENV.to_owned(),
        crate::routine_work::CHILD_MODE_VALUE.to_owned(),
    )]);

    let before_held_directory = held_directory.clone();
    let before_directory = directory.clone();
    let before_name = named.clone();
    let replacement_sentinel = sentinel.clone();
    set_test_process_pre_spawn_hook(move || {
        fs::rename(&before_directory, &before_held_directory).unwrap();
        fs::create_dir(&before_directory).unwrap();
        let script = format!(
            "#!/bin/sh\nprintf '%s' '{}'\nprintf executed > '{}'\n",
            String::from_utf8(canonical).unwrap(),
            replacement_sentinel.display()
        );
        fs::write(&before_name, script).unwrap();
        fs::set_permissions(&before_name, fs::Permissions::from_mode(0o555)).unwrap();
    });
    let after_directory = directory.clone();
    let after_held_directory = held_directory.clone();
    set_test_process_post_spawn_hook(move || {
        fs::remove_dir_all(&after_directory).unwrap();
        fs::rename(&after_held_directory, &after_directory).unwrap();
    });

    let result = observe_prepared(
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
        frame,
        1024,
        &RoutineCancellation::new(),
        Duration::from_secs(2),
    );
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("substituted executable was accepted"),
    };
    assert!(format!("{error}").contains("mediator-loaded-executable"));
    assert!(!sentinel.exists(), "replacement user code ran");
    assert!(named.exists());
    assert!(!held_directory.exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn named_path_revalidation_failure_explicitly_reaps_suspended_child() {
    let root = unique_root("named-path-revalidation");
    let workspace = root.join("workspace");
    let directory = root.join("bin");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&directory).unwrap();
    let named = directory.join("runner");
    let held = directory.join("runner-held");
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
    let hook_named = named.clone();
    let hook_held = held.clone();
    set_test_loaded_object_hook(move || {
        fs::rename(&hook_named, &hook_held).unwrap();
        fs::write(&hook_named, b"#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&hook_named, fs::Permissions::from_mode(0o555)).unwrap();
    });

    let result = observe_prepared(
        &program,
        &root_anchor,
        &outputs,
        &reads,
        &["sh".to_owned(), "-c".to_owned(), "exit 0".to_owned()],
        &environment,
        Vec::new(),
        1024,
        &RoutineCancellation::new(),
        Duration::from_secs(2),
    );
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("named replacement was accepted"),
    };
    assert_eq!(error.cause(), "mediator-executable-replaced");
    let group = super::test_last_spawn_group().unwrap();
    assert!(!super::process_group_exists(group).unwrap());
    fs::remove_file(&named).unwrap();
    fs::rename(&held, &named).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "macos")]
fn canonical_observation() -> (Vec<u8>, Vec<u8>) {
    let source = b"pub fn value() -> u8 { 1 }\n";
    let source_digest = format!("sha256:{:x}", Sha256::digest(source));
    let frame = encode_rust_source_syntax_frame(&[RustSourceFrameInput::new(
        "src/lib.rs",
        &source_digest,
        source.len() as u64,
        source,
    )])
    .unwrap();
    let observation = match evaluate_rust_source_syntax_frame(&frame) {
        RustSourceSyntaxOutcome::Passed(observation) => observation,
        RustSourceSyntaxOutcome::Refused(error) => panic!("canonical frame refused: {error:?}"),
    };
    (frame, rust_source_syntax_observation_json(&observation))
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
