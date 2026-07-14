#[test]
fn production_boundary_has_one_sealed_issuer_and_no_test_grant_entrypoint() {
    let runtime = include_str!("../src/routine_work/runtime_adapter/mod.rs");
    let production = include_str!("../src/routine_work/runtime_adapter/production/mod.rs");
    let issuance =
        include_str!("../src/routine_work/runtime_adapter/production/production_issuance.rs");
    let recovery =
        include_str!("../src/routine_work/runtime_adapter/production/recovery_authority.rs");
    let authority =
        include_str!("../src/routine_work/runtime_adapter/production/ledger/authority_record.rs");
    let cancellation = include_str!(
        "../src/routine_work/runtime_adapter/mediator/outcome/routine_cancellation.rs"
    );
    assert_eq!(issuance.matches("issue_production_grant(").count(), 1);
    assert!(recovery.contains("pub(crate) struct ProductionRoutineIssuer"));
    assert!(production.contains("preflight_production_request"));
    assert!(!production.contains("RoutineRootGrant::test_issue"));
    assert!(cancellation.contains("#[cfg(test)]\nimpl RoutineRootGrant"));
    assert!(runtime.contains("mod production;"));
    assert!(!production.contains("ClaimDecision"));
    assert!(!production.contains("public command"));
    assert!(authority.contains("pub(crate) struct ReusePreauthorization"));
}

#[test]
fn production_source_exposes_no_arbitrary_process_binding_surface() {
    let selection = include_str!("../src/routine_work/runtime_adapter/selection_limit.rs");
    let invocation = include_str!("../src/routine_work/runtime_adapter/invocation_binding.rs");
    let mediation =
        include_str!("../src/routine_work/runtime_adapter/mediator/intent_mediation.rs");
    let process =
        include_str!("../src/routine_work/runtime_adapter/mediator/process/process_execution.rs");
    let runner = include_str!("../src/routine_work/runtime_adapter/runner_binding.rs");
    let reconciliation =
        include_str!("../src/routine_work/runtime_adapter/execution_reconciliation.rs");
    let grant = include_str!("../src/routine_work/runtime_adapter/mediator/grant_validation.rs");

    assert!(!selection.contains("external-process-exit-v1"));
    assert!(!selection.contains("bind_routine_invocation("));
    assert!(!selection.contains("bind_routine_invocation_with_environment"));
    assert!(selection.contains("bind_rust_source_syntax_invocation("));
    assert!(invocation.contains("RUST_SOURCE_SYNTAX_ARGUMENTS"));
    assert!(invocation.contains("adapter-rust-source-input-missing"));
    assert!(!mediation.contains("framed_input.as_ref()"));
    assert!(!mediation.contains("unwrap_or(\"none\")"));
    assert!(
        mediation.find("validate_rust_source_observation").unwrap()
            < mediation.find("ResultArtifactWire").unwrap()
    );
    assert!(process.contains("framed_input: Vec<u8>"));
    assert!(!process.contains("framed_input: Option"));
    assert!(process.contains("stdin(Stdio::piped())"));
    assert!(reconciliation.contains("adapter-current-runner-substituted"));
    assert!(runner.contains("invocation.environment != expected_environment"));
    assert!(grant.contains("intent.environment() != &expected_environment"));
}

#[test]
fn publication_is_staged_before_cache_and_terminal_settlement() {
    let mediation = include_str!("../src/routine_work/runtime_adapter/mediator/no_op_mediation.rs");
    let output = include_str!(
        "../src/routine_work/runtime_adapter/mediator/filesystem/ownership_rejection.rs"
    );
    let reservation =
        include_str!("../src/routine_work/runtime_adapter/mediator/read_source_binding.rs");
    let stage = mediation.find("attempt.stage_success").unwrap();
    let publish = mediation.find("publisher.publish").unwrap();
    let settle = mediation.find("attempt.settle_success").unwrap();
    assert!(stage < publish && publish < settle);
    assert!(output.contains("mediator-output-scope-not-empty"));
    assert!(output.contains("capture_owned_delta"));
    assert!(output.contains("held != scope.identity"));
    let incomplete = reservation.find("pub(crate) fn settle_incomplete").unwrap();
    let drop = reservation[incomplete..].find("impl Drop").unwrap();
    assert!(!reservation[incomplete..incomplete + drop].contains("durable.settle"));
}

#[test]
fn child_behavior_requires_a_durable_process_bound_one_use_channel() {
    let child = include_str!("../src/cli/successor_public/routine/behavior_child.rs");
    let capability = include_str!("../src/routine_work/behavior/child_capability.rs");
    let mediation =
        include_str!("../src/routine_work/runtime_adapter/mediator/intent_mediation.rs");
    let channel = include_str!(
        "../src/routine_work/runtime_adapter/mediator/process/child_authority_channel.rs"
    );
    let process =
        include_str!("../src/routine_work/runtime_adapter/mediator/process/process_execution.rs");
    for obsolete in [
        "HUL_ROUTINE_REQUEST_ID",
        "HUL_ROUTINE_PROTOCOL_ID",
        "HUL_ROUTINE_INTENT_ID",
        "HUL_ROUTINE_NODE_ID",
        "HUL_ROUTINE_BEHAVIOR_ID",
    ] {
        assert!(!child.contains(obsolete));
        assert!(!mediation.contains(obsolete));
    }
    let validate = child.find("validate_capability(invocation").unwrap();
    let framed_input = child.find("std::io::stdin()").unwrap();
    assert!(validate < framed_input);
    for binding in [
        "request_id",
        "protocol_id",
        "intent_id",
        "node_id",
        "grant_session_id",
        "grant_id",
        "reservation_marker",
        "parent_pid",
        "child_pid",
        "process_session_id",
        "program_path_hex",
        "program_sha256",
    ] {
        assert!(capability.contains(binding), "missing binding {binding}");
    }
    assert!(child.contains("LOCAL_PEERPID"));
    assert!(child.contains("process_path(parent_pid)"));
    assert!(channel.contains("capability.acknowledgement()"));
    assert!(channel.contains("getrandom::fill"));
    assert!(channel.contains("capability.seal(binding.secret)"));
    assert!(mediation.contains("attempt.prepare_spawn()?"));
    assert!(
        process.find("child_authority.authorize").unwrap() < process.find("on_started()?").unwrap()
    );
}
