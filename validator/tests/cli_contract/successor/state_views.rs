use super::super::successor::runtime::{
    RuntimeSession, StateDisposition, StateProjection, StateView,
};
use super::super::successor::{EffectClass, OptionArgument, OptionName, OutputMode, ParsedValue};
use super::{Repository, context, parsed, tree_snapshot};
use serde_json::json;
use std::fs;

struct FakeState {
    context_id: String,
    state_id: &'static str,
    findings: usize,
    disposition: StateDisposition,
}

impl StateView for FakeState {
    fn context_id(&self) -> &str {
        &self.context_id
    }

    fn state_id(&self) -> &str {
        self.state_id
    }

    fn finding_count(&self) -> usize {
        self.findings
    }

    fn disposition(&self) -> StateDisposition {
        self.disposition
    }

    fn project(&self, projection: StateProjection<'_>) -> Result<Option<Vec<u8>>, String> {
        if matches!(projection, StateProjection::Diagnose(Some("absent"))) {
            return Ok(None);
        }
        let schema = match projection {
            StateProjection::Summary => "ProductStateSummary-v1",
            StateProjection::Findings => "ProductStateFindings-v1",
            StateProjection::Claims => "ProductStateClaims-v1",
            StateProjection::Diagnose(_) => "ProductStateDiagnose-v1",
            StateProjection::Next => "ProductStateNext-v1",
        };
        serde_json::to_vec(&json!({
            "schema_version": schema,
            "state_id": self.state_id,
            "context_id": self.context_id,
            "finding_count": self.findings,
        }))
        .map(Some)
        .map_err(|_| "fake projection failed".to_owned())
    }
}

#[test]
fn one_context_drives_state_views_stable_streams_and_exact_exit_classes_without_writes() {
    let repository = Repository::new("state-view-runtime");
    fs::write(repository.root.join("dirty.txt"), b"dirty\n").unwrap();
    let context = context(&repository.root);
    let state = FakeState {
        context_id: context.context_id().to_owned(),
        state_id: "sha256:fake-state",
        findings: 1,
        disposition: StateDisposition::NoLegalRoute,
    };
    let session = RuntimeSession::new(&context, Some(&state));
    let before_tree = tree_snapshot(&repository.root);
    let before_status = repository.status();

    let context_streams = session
        .dispatch(&parsed(&["--json", "inspect", "context"]))
        .render(OutputMode::Json);
    assert_eq!(context_streams.exit_code, 0);
    assert!(context_streams.stderr.is_empty());
    assert_versioned(&context_streams.stdout, "LiveContext-v1");

    for command in [
        vec!["--json", "inspect"],
        vec!["--json", "inspect", "findings"],
        vec!["--json", "inspect", "claims"],
        vec!["--json", "diagnose"],
    ] {
        let streams = session.dispatch(&parsed(&command)).render(OutputMode::Json);
        assert_eq!(streams.exit_code, 1);
        assert!(streams.stderr.is_empty());
        assert!(serde_json::from_slice::<serde_json::Value>(&streams.stdout).is_ok());
    }

    let next = session
        .dispatch(&parsed(&["--json", "next"]))
        .render(OutputMode::Json);
    assert_eq!(next.exit_code, 3);
    assert!(next.stderr.is_empty());
    assert_versioned(&next.stdout, "ProductStateNext-v1");

    let downstream = session
        .dispatch(&parsed(&["--json", "fit", "plan"]))
        .render(OutputMode::Json);
    assert_eq!(downstream.exit_code, 4);
    assert!(downstream.stdout.is_empty());
    assert_diagnostic(
        &downstream.stderr,
        "successor_runtime_downstream_tool_unavailable",
    );

    let mut malformed = parsed(&["--json", "inspect", "context"]);
    malformed.arguments.push(OptionArgument {
        name: OptionName::ApproveExport,
        value: ParsedValue::Flag,
    });
    let invalid = session.dispatch(&malformed).render(OutputMode::Json);
    assert_eq!(invalid.exit_code, 2);
    assert_diagnostic(&invalid.stderr, "successor_runtime_unexpected_arguments");

    let mut drift = parsed(&["--json", "inspect", "context"]);
    drift.effect = EffectClass::WorkspaceWrite;
    let internal = session.dispatch(&drift).render(OutputMode::Json);
    assert_eq!(internal.exit_code, 70);
    assert_diagnostic(&internal.stderr, "successor_runtime_effect_mismatch");

    assert_eq!(tree_snapshot(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
}

#[test]
fn missing_mismatched_and_unknown_state_fail_closed_without_input_echo() {
    let repository = Repository::new("state-view-failures");
    let context = context(&repository.root);
    let no_state = RuntimeSession::new(&context, None)
        .dispatch(&parsed(&["--json", "next"]))
        .render(OutputMode::Json);
    assert_eq!(no_state.exit_code, 4);
    assert_diagnostic(&no_state.stderr, "successor_runtime_state_unavailable");

    let mismatched = FakeState {
        context_id: "sha256:other-candidate-canary".to_owned(),
        state_id: "sha256:wrong-state",
        findings: 0,
        disposition: StateDisposition::NoAction,
    };
    let streams = RuntimeSession::new(&context, Some(&mismatched))
        .dispatch(&parsed(&["--json", "inspect"]))
        .render(OutputMode::Json);
    assert_eq!(streams.exit_code, 70);
    assert_diagnostic(&streams.stderr, "successor_runtime_state_context_mismatch");
    assert!(!String::from_utf8_lossy(&streams.stderr).contains("other-candidate-canary"));

    let state = FakeState {
        context_id: context.context_id().to_owned(),
        state_id: "sha256:current-state",
        findings: 1,
        disposition: StateDisposition::Action,
    };
    let missing = RuntimeSession::new(&context, Some(&state))
        .dispatch(&parsed(&["--json", "diagnose", "--finding", "absent"]))
        .render(OutputMode::Json);
    assert_eq!(missing.exit_code, 1);
    assert!(missing.stdout.is_empty());
    assert_diagnostic(&missing.stderr, "successor_runtime_finding_not_present");
    assert!(!String::from_utf8_lossy(&missing.stderr).contains("absent"));
}

fn assert_versioned(bytes: &[u8], expected: &str) {
    assert!(!bytes.contains(&0x1b));
    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(value["schema_version"], expected);
}

fn assert_diagnostic(bytes: &[u8], expected: &str) {
    assert!(!bytes.contains(&0x1b));
    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(value["diagnostic_id"], expected);
}
