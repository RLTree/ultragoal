use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::symlink;
use std::process::Command;

use super::routine_plan_fixture::{RoutinePlanFixture, authority_path, repo_path};
use super::routine_work::{
    PreparedRoutineExecution, RoutineCancellation, RoutineReuseInput,
    mediate_public_routine_execution,
};

fn cause<T>(result: Result<T, super::routine_work::RoutineError>) -> &'static str {
    result.err().expect("mutation must refuse").cause()
}

#[test]
fn closed_binding_refuses_loader_child_argv_program_and_policy_mutations() {
    if isolate(
        "local_issuer_binding_controls::closed_binding_refuses_loader_child_argv_program_and_policy_mutations",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("typed-binding-mutations");
    let replace_first = |first| {
        let mut invocations = fixture.invocations();
        invocations[0] = first;
        cause(fixture.prepare_with(invocations))
    };
    let fresh = || fixture.invocation(fixture.plan.checks()[0].node_id());

    for key in ["DYLD_INSERT_LIBRARIES", "LD_PRELOAD", "BASH_ENV"] {
        let mut environment = BTreeMap::from([
            ("LANG".to_owned(), "C".to_owned()),
            ("LC_ALL".to_owned(), "C".to_owned()),
            ("PATH".to_owned(), "/usr/bin".to_owned()),
        ]);
        environment.insert(key.to_owned(), "/tmp/attacker".to_owned());
        assert_eq!(
            replace_first(fresh().test_with_environment(environment)),
            "adapter-runner-binding-mismatch"
        );
    }
    let mut child_environment = fresh().environment.clone();
    child_environment.insert("HUL_ROUTINE_CHILD_FD".to_owned(), "198".to_owned());
    assert_eq!(
        replace_first(fresh().test_with_environment(child_environment)),
        "adapter-runner-binding-mismatch"
    );
    assert_eq!(
        replace_first(fresh().test_with_arguments(vec!["--arbitrary".to_owned()])),
        "adapter-runner-binding-mismatch"
    );
    assert_eq!(
        replace_first(fresh().test_with_program_path_hex("2f746d702f61747461636b6572")),
        "adapter-runner-binding-mismatch"
    );
    assert_eq!(
        replace_first(fresh().test_with_program_sha256(format!("sha256:{}", "0".repeat(64)))),
        "adapter-runner-binding-mismatch"
    );
    assert_eq!(
        replace_first(fresh().test_with_behavior_id("external-process-exit-v1")),
        "adapter-runner-binding-mismatch"
    );
    assert_eq!(
        replace_first(fresh().test_with_timeout_ms(0)),
        "adapter-timeout-invalid"
    );
    assert!(matches!(
        fixture.prepare().unwrap(),
        PreparedRoutineExecution::Effect(_)
    ));
    fixture.finish();
}

#[test]
fn read_source_alias_special_and_mutate_restore_refuse_before_effect() {
    if isolate(
        "local_issuer_binding_controls::read_source_alias_special_and_mutate_restore_refuse_before_effect",
    ) {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("bound-read-refusals");
    let source = fixture.repo.root().join("src/lib.rs");

    symlink("lib.rs", fixture.repo.root().join("src/link.rs")).unwrap();
    assert!(
        super::routine_work::bind_rust_source_syntax_invocation(
            &fixture.context,
            &fixture.plan,
            fixture.plan.checks()[0].node_id(),
            vec![repo_path("src/link.rs")],
            60_000,
            1024,
            vec![repo_path("target/routine")],
        )
        .is_err()
    );
    fs::remove_file(fixture.repo.root().join("src/link.rs")).unwrap();

    fs::hard_link(&source, fixture.repo.root().join("src/hard.rs")).unwrap();
    assert!(
        super::routine_work::bind_rust_source_syntax_invocation(
            &fixture.context,
            &fixture.plan,
            fixture.plan.checks()[0].node_id(),
            vec![repo_path("src/hard.rs")],
            60_000,
            1024,
            vec![repo_path("target/routine")],
        )
        .is_err()
    );
    fs::remove_file(fixture.repo.root().join("src/hard.rs")).unwrap();

    let invocations = fixture.invocations();
    let original = fs::read(&source).unwrap();
    fs::write(&source, b"same length mutation________\n").unwrap();
    fs::write(&source, &original).unwrap();
    assert_eq!(
        cause(fixture.prepare_with(invocations)),
        "mediator-read-source-binding-stale"
    );
    fixture.finish();
}

#[test]
fn context_mutation_refuses_before_authority_creation() {
    if isolate("local_issuer_binding_controls::context_mutation_refuses_before_authority_creation")
    {
        return;
    }
    let mut fixture = RoutinePlanFixture::new("context-mutation");
    let prepared = fixture.prepare().unwrap();
    let authority = authority_path(&fixture);
    fixture.repo.write("src/lib.rs", b"context mutation\n");
    let error = mediate_public_routine_execution(
        Some(&authority),
        &fixture.context,
        &fixture.plan,
        prepared,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        error.cause(),
        "mediator-context-preflight-stale" | "mediator-dirty-snapshot-stale"
    ));
    assert!(!authority.exists());
    fixture.finish();
}

fn isolate(test: &str) -> bool {
    const CHILD: &str = "HUL_ROUTINE_LOCAL_ISSUER_CONTROL_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return false;
    }
    let status = Command::new(std::env::current_exe().unwrap())
        .args([test, "--exact", "--test-threads=1"])
        .env(CHILD, "1")
        .status()
        .unwrap();
    assert!(
        status.success(),
        "isolated local issuer control failed: {test}"
    );
    true
}
