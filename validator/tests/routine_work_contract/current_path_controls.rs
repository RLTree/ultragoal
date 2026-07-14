use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::symlink;

use super::current_path_fixture::{CurrentPathFixture, authority_path, repo_path};
use super::routine_work::{
    PreparedRoutineExecution, RoutineCancellation, RoutineInvocationSpec, RoutineReuseInput,
    mediate_prepared_routine_execution_production, test_probe_execute_without_root_broker,
    test_spawn_count,
};

enum IgnoredBinding {
    Environment,
    Arguments,
    ProgramPath,
    ProgramDigest,
    Behavior,
    Timeout,
}

fn enforcement_disabled_mutant_accepts(
    mut attacker: RoutineInvocationSpec,
    expected: RoutineInvocationSpec,
    ignored: IgnoredBinding,
) -> bool {
    match ignored {
        IgnoredBinding::Environment => attacker.environment = expected.environment.clone(),
        IgnoredBinding::Arguments => attacker.arguments = expected.arguments.clone(),
        IgnoredBinding::ProgramPath => {
            attacker.program_path_hex = expected.program_path_hex.clone()
        }
        IgnoredBinding::ProgramDigest => attacker.program_sha256 = expected.program_sha256.clone(),
        IgnoredBinding::Behavior => attacker.behavior_id = expected.behavior_id.clone(),
        IgnoredBinding::Timeout => attacker.timeout_ms = expected.timeout_ms,
    }
    attacker == expected
}

fn cause<T>(result: Result<T, super::routine_work::RoutineError>) -> &'static str {
    result.err().expect("mutation must refuse").cause()
}

#[test]
fn closed_binding_refuses_loader_child_argv_program_and_policy_mutations() {
    let fixture = CurrentPathFixture::new("current-binding-mutations");
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
}

#[test]
fn single_field_enforcement_disabled_mutants_accept_each_rejected_attack() {
    let fixture = CurrentPathFixture::new("binding-mutant-sensitivity");
    let valid = || fixture.invocation(fixture.plan.checks()[0].node_id());
    let mut loader = valid().environment.clone();
    loader.insert(
        "DYLD_INSERT_LIBRARIES".to_owned(),
        "/tmp/attacker".to_owned(),
    );
    assert!(enforcement_disabled_mutant_accepts(
        valid().test_with_environment(loader),
        valid(),
        IgnoredBinding::Environment,
    ));
    assert!(enforcement_disabled_mutant_accepts(
        valid().test_with_arguments(vec!["--arbitrary".to_owned()]),
        valid(),
        IgnoredBinding::Arguments,
    ));
    assert!(enforcement_disabled_mutant_accepts(
        valid().test_with_program_path_hex("2f746d702f61747461636b6572"),
        valid(),
        IgnoredBinding::ProgramPath,
    ));
    assert!(enforcement_disabled_mutant_accepts(
        valid().test_with_program_sha256(format!("sha256:{}", "0".repeat(64))),
        valid(),
        IgnoredBinding::ProgramDigest,
    ));
    assert!(enforcement_disabled_mutant_accepts(
        valid().test_with_behavior_id("external-process-exit-v1"),
        valid(),
        IgnoredBinding::Behavior,
    ));
    assert!(enforcement_disabled_mutant_accepts(
        valid().test_with_timeout_ms(0),
        valid(),
        IgnoredBinding::Timeout,
    ));
}

#[test]
fn read_source_alias_special_and_mutate_restore_refuse_before_effect() {
    let fixture = CurrentPathFixture::new("current-read-refusals");
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
}

#[test]
fn context_mutation_refuses_before_authority_creation() {
    let fixture = CurrentPathFixture::new("current-context-mutation");
    let prepared = fixture.prepare().unwrap();
    let authority = authority_path(&fixture);
    fixture.repo.write("src/lib.rs", b"context mutation\n");
    let error = mediate_prepared_routine_execution_production(
        &authority,
        &fixture.context,
        &fixture.plan,
        prepared,
        None,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert!(matches!(
        error.cause(),
        "mediator-context-preflight-stale" | "mediator-dirty-snapshot-stale"
    ));
    assert!(!authority.exists());
}

#[test]
fn root_broker_gate_refuses_before_spawn_and_writes() {
    let fixture = CurrentPathFixture::new("root-broker-pre-spawn-gate");
    let authority = authority_path(&fixture);
    let before = fixture.repo.tree();
    assert_eq!(test_spawn_count(), 0);
    let root = fs::canonicalize(fixture.repo.root()).unwrap();
    assert_eq!(
        test_probe_execute_without_root_broker(
            &root,
            std::path::Path::new("/usr/bin/true"),
            &[repo_path("target/routine")],
            &[repo_path("src/lib.rs")],
        )
        .unwrap_err()
        .cause(),
        "mediator-child-root-broker-required"
    );
    assert_eq!(test_spawn_count(), 0);
    assert_eq!(fixture.repo.tree(), before);
    assert!(!authority.exists());
}
