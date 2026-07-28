use std::collections::BTreeMap;

use super::super::context::{BuildRequest, LiveContext};
use super::super::routine_work::{
    ReuseDecision, ReuseMiss, ReuseReceipt, RoutineErrorId, RunOutcome, assess_reuse,
};
use super::super::scenario::{TempRepo, sha};
use super::execution_fixture::{
    authority_context, authority_plan, capture_bytes, capture_guard, capture_receipt,
    observe_bytes, result_bytes_with, syntax_evidence,
};

#[cfg(target_os = "macos")]
#[test]
fn exact_anchored_evidence_reuses_without_reexecuting_work() {
    let _capture = capture_guard();
    let mut repo = TempRepo::new("reuse-hit");
    let (context, plan) = authority_plan(&repo);
    let (expectation, _, receipt, observed) = syntax_evidence(&repo, &context, &plan);
    for _ in 0..2 {
        assert!(matches!(
            assess_reuse(&context, &expectation, &receipt, &observed).unwrap(),
            ReuseDecision::Hit(_)
        ));
    }
    repo.teardown_after_assertions();
}

#[cfg(target_os = "macos")]
#[test]
fn fresh_live_configuration_input_and_tool_substitutions_miss() {
    let _capture = capture_guard();
    let mut repo = TempRepo::new("reuse-live-dimensions");
    let (context, plan) = authority_plan(&repo);
    let (expectation, _, receipt, observed) = syntax_evidence(&repo, &context, &plan);
    let build = || {
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", "routine")
            .probe_tool("sandbox-exec")
            .probe_tool("true")
    };
    let configuration = LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", "different")
            .probe_tool("sandbox-exec")
            .probe_tool("true"),
    )
    .unwrap();
    let inputs = LiveContext::build(build().select_input("src/lib.rs")).unwrap();
    let tools = LiveContext::build(build().probe_tool("false")).unwrap();
    for (current, expected) in [
        (&configuration, ReuseMiss::Configuration),
        (&inputs, ReuseMiss::SelectedInputs),
        (&tools, ReuseMiss::ToolSet),
    ] {
        assert_eq!(
            assess_reuse(current, &expectation, &receipt, &observed).unwrap(),
            ReuseDecision::Miss(expected)
        );
    }
    repo.teardown_after_assertions();
}

#[cfg(target_os = "macos")]
#[test]
fn every_context_plan_check_scope_tool_and_input_dimension_is_exact() {
    let _capture = capture_guard();
    let mut repo = TempRepo::new("reuse-dimensions");
    let (context, plan) = authority_plan(&repo);
    let (expectation, execution, _, observed) = syntax_evidence(&repo, &context, &plan);
    let receipt = execution.receipt_json();
    let cases = [
        ("context_id", sha(b"context"), ReuseMiss::Context),
        ("candidate_id", sha(b"candidate"), ReuseMiss::Candidate),
        ("root_id", sha(b"root"), ReuseMiss::Root),
        (
            "configuration_id",
            sha(b"configuration"),
            ReuseMiss::Configuration,
        ),
        (
            "selected_inputs_id",
            sha(b"inputs"),
            ReuseMiss::SelectedInputs,
        ),
        ("tool_set_id", sha(b"tool-set"), ReuseMiss::ToolSet),
        ("graph_id", sha(b"graph"), ReuseMiss::Graph),
        ("plan_id", sha(b"plan"), ReuseMiss::Plan),
        ("coverage_id", sha(b"coverage"), ReuseMiss::Coverage),
        ("node_id", "other".to_owned(), ReuseMiss::Node),
        ("tool_name", "false".to_owned(), ReuseMiss::Tool),
        ("tool_identity", sha(b"tool"), ReuseMiss::Tool),
        ("tool_program_path_hex", "00".to_owned(), ReuseMiss::Tool),
        ("input_id", sha(b"input"), ReuseMiss::Input),
        (
            "result_scope",
            "different".to_owned(),
            ReuseMiss::ResultScope,
        ),
    ];
    for (index, (field, replacement, expected)) in cases.into_iter().enumerate() {
        let changed = mutate_string(receipt, field, &replacement);
        if matches!(field, "context_id" | "candidate_id") {
            let artifact =
                capture_bytes(&repo, &context, &changed, &format!("changed-{index}")).unwrap();
            assert_eq!(
                ReuseReceipt::from_captured(&artifact).unwrap_err().id(),
                super::super::routine_work::RoutineErrorId::InvalidReceipt,
                "{field}"
            );
            continue;
        }
        let receipt = capture_receipt(&repo, &context, &changed, &format!("changed-{index}"));
        assert_eq!(
            assess_reuse(&context, &expectation, &receipt, &observed).unwrap(),
            ReuseDecision::Miss(expected),
            "{field}"
        );
    }
    repo.teardown_after_assertions();
}

#[cfg(target_os = "macos")]
#[test]
fn stale_candidate_and_incomplete_failed_or_unobserved_receipts_miss() {
    let _capture = capture_guard();
    let mut repo = TempRepo::new("reuse-stale-state");
    let (context, plan) = authority_plan(&repo);
    let (expectation, execution, receipt, observed) = syntax_evidence(&repo, &context, &plan);
    let started = ReuseReceipt::started_bytes(&expectation).unwrap();
    let started = capture_receipt(&repo, &context, &started, "started");
    assert_eq!(
        assess_reuse(&context, &expectation, &started, &observed).unwrap(),
        ReuseDecision::Miss(ReuseMiss::Incomplete)
    );
    for (field, before, after, expected) in [
        (
            "outcome",
            "\"passed\"",
            "\"failed\"",
            ReuseMiss::FailedResult,
        ),
        (
            "behavior_observed",
            "true",
            "false",
            ReuseMiss::BehaviorNotObserved,
        ),
    ] {
        let bytes = replace_scalar(execution.receipt_json(), field, before, after);
        let receipt = capture_receipt(&repo, &context, &bytes, field);
        assert_eq!(
            assess_reuse(&context, &expectation, &receipt, &observed).unwrap(),
            ReuseDecision::Miss(expected)
        );
    }
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 8 }\n");
    assert_eq!(
        assess_reuse(&context, &expectation, &receipt, &observed)
            .unwrap_err()
            .id(),
        RoutineErrorId::ConcurrentMutation
    );
    let current = authority_context(&repo);
    assert_eq!(
        assess_reuse(&current, &expectation, &receipt, &observed).unwrap(),
        ReuseDecision::Miss(ReuseMiss::Candidate)
    );
    repo.teardown_after_assertions();
}

#[cfg(target_os = "macos")]
#[test]
fn content_and_same_size_output_substitution_never_hit() {
    let _capture = capture_guard();
    let mut repo = TempRepo::new("reuse-substitution");
    let (context, plan) = authority_plan(&repo);
    let (expectation, _, receipt, _) = syntax_evidence(&repo, &context, &plan);
    let behavior = result_bytes_with(
        &context,
        &expectation,
        RunOutcome::Passed,
        true,
        sha(b"different-behavior"),
        BTreeMap::from([("artifact".to_owned(), sha(b"artifact"))]),
    );
    let observed = observe_bytes(&repo, &context, &expectation, &behavior);
    assert_eq!(
        assess_reuse(&context, &expectation, &receipt, &observed).unwrap(),
        ReuseDecision::Miss(ReuseMiss::ResultSubstitution)
    );
    let output = result_bytes_with(
        &context,
        &expectation,
        RunOutcome::Passed,
        true,
        sha(b"syntax-behavior"),
        BTreeMap::from([("artifact".to_owned(), sha(b"artifacu"))]),
    );
    assert_eq!(behavior.len(), output.len());
    let observed = observe_bytes(&repo, &context, &expectation, &output);
    assert_eq!(
        assess_reuse(&context, &expectation, &receipt, &observed).unwrap(),
        ReuseDecision::Miss(ReuseMiss::OutputSubstitution)
    );
    repo.teardown_after_assertions();
}

fn mutate_string(bytes: &[u8], field: &str, replacement: &str) -> Vec<u8> {
    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let current = value["binding"][field].as_str().unwrap();
    String::from_utf8(bytes.to_vec())
        .unwrap()
        .replacen(
            &format!("\"{field}\":\"{current}\""),
            &format!("\"{field}\":\"{replacement}\""),
            1,
        )
        .into_bytes()
}

fn replace_scalar(bytes: &[u8], field: &str, before: &str, after: &str) -> Vec<u8> {
    String::from_utf8(bytes.to_vec())
        .unwrap()
        .replacen(
            &format!("\"{field}\":{before}"),
            &format!("\"{field}\":{after}"),
            1,
        )
        .into_bytes()
}
