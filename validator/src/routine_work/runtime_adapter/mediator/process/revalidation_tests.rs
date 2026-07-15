#[cfg(target_os = "macos")]
use super::super::filesystem::{OutputConfinement, PinnedExecutable, ReadConfinement, RootAnchor};
#[cfg(target_os = "macos")]
use super::super::outcome::RoutineCancellation;
#[cfg(target_os = "macos")]
use super::process_termination_tests::ProcessFixture;
#[cfg(target_os = "macos")]
use super::*;
#[cfg(target_os = "macos")]
use crate::routine_work::RepoPath;
#[cfg(target_os = "macos")]
use std::collections::BTreeMap;
#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(target_os = "macos")]
#[test]
fn root_revalidation_failure_reaps_before_returning() {
    let fixture = ProcessFixture::new("root-revalidation");
    let replacement = fixture.root.join("workspace-replacement");
    let workspace = fixture.workspace.clone();
    let hook_workspace = workspace.clone();
    let hook_replacement = replacement.clone();
    set_test_process_post_spawn_hook(move || {
        fs::rename(&hook_workspace, &hook_replacement).unwrap();
        fs::create_dir(&hook_workspace).unwrap();
    });
    let result = fixture.run_result(
        "while :; do :; done",
        Duration::from_secs(2),
        1024,
        &RoutineCancellation::new(),
        || Ok(()),
    );
    assert!(result.is_err());
    assert_last_group_absent();
    fs::remove_dir(&workspace).unwrap();
    fs::rename(&replacement, &workspace).unwrap();
    fixture.teardown();
}

#[cfg(target_os = "macos")]
#[test]
fn output_revalidation_failure_reaps_before_returning() {
    let fixture = ProcessFixture::new("output-revalidation");
    let output = fixture.workspace.join("out");
    let held = fixture.workspace.join("out-held");
    fs::create_dir(&output).unwrap();
    let root = RootAnchor::open(&fixture.workspace).unwrap();
    let outputs =
        OutputConfinement::prepare(&root, &[RepoPath::parse("out").unwrap()], 1024).unwrap();
    let reads = ReadConfinement {
        sources: Vec::new(),
    };
    let program = PinnedExecutable::open_unbound(std::path::Path::new("/bin/sh")).unwrap();
    let environment = BTreeMap::from([(
        crate::routine_work::CHILD_MODE_ENV.to_owned(),
        crate::routine_work::CHILD_MODE_VALUE.to_owned(),
    )]);
    let hook_output = output.clone();
    let hook_held = held.clone();
    set_test_process_post_spawn_hook(move || {
        fs::rename(&hook_output, &hook_held).unwrap();
        fs::create_dir(&hook_output).unwrap();
    });
    let result = execute(
        &program,
        &root,
        &outputs,
        &reads,
        &["sh".to_owned(), "-c".to_owned(), "exit 0".to_owned()],
        &environment,
        Vec::new(),
        Duration::from_secs(2),
        1024,
        &RoutineCancellation::new(),
        || Ok(()),
    );
    assert!(result.is_err());
    assert_last_group_absent();
    fs::remove_dir(&output).unwrap();
    fs::rename(&held, &output).unwrap();
    fixture.teardown();
}

#[cfg(target_os = "macos")]
#[test]
fn setup_panic_preserves_payload_after_explicit_reap() {
    let fixture = ProcessFixture::new("setup-panic");
    set_test_process_post_spawn_hook(|| std::panic::panic_any("process-setup-panic"));
    let payload = match catch_unwind(AssertUnwindSafe(|| {
        fixture.run_result(
            "while :; do :; done",
            Duration::from_secs(2),
            1024,
            &RoutineCancellation::new(),
            || Ok(()),
        )
    })) {
        Err(payload) => payload,
        Ok(_) => panic!("setup panic was swallowed"),
    };
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"process-setup-panic"));
    assert_last_group_absent();
    fixture.teardown();
}

#[cfg(target_os = "macos")]
fn assert_last_group_absent() {
    let group = test_last_spawn_group().expect("spawned process group was not observed");
    assert!(!process_group_exists(group).unwrap());
}
