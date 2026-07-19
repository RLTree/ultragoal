use super::*;
use crate::cli::successor::{
    EffectClass, InspectTarget, OptionArgument, OptionName, ParsedInvocation, ParsedValue,
    SuccessorCommand,
};
use crate::cli::successor_public::strict;

#[test]
pub(crate) fn orchestration_reads_the_canonical_frontier_without_writes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repository parent");
    let before_tree =
        strict::zero_write_guard::capture(root).expect("capture bounded repository state");
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "inspect", "orchestration"]).expect("orchestration route parses")
    else {
        panic!("orchestration route invokes");
    };

    let outcome = execute_invocation(root, invocation);
    let streams = outcome.render(OutputMode::Json);

    assert_eq!(streams.exit_code, 0);
    assert!(streams.stderr.is_empty());
    let output: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(
        output["schema_version"],
        "HarnessOrchestrationInspection-v1"
    );
    assert!(
        output["registry_sha256"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert!(output["lanes"].as_array().unwrap().iter().all(|lane| {
        lane.get("id").is_some()
            && lane.get("lifecycle").is_some()
            && lane.get("owner").is_none()
            && lane.get("worktree").is_none()
    }));
    let human = outcome.render(OutputMode::Human);
    assert_eq!(human.exit_code, 0);
    assert!(human.stderr.is_empty());
    for rendered in [
        String::from_utf8(streams.stdout).unwrap(),
        String::from_utf8(human.stdout).unwrap(),
    ] {
        assert_forbidden_output(&rendered, root);
    }
    assert_eq!(
        strict::zero_write_guard::capture(root).expect("recapture bounded repository state"),
        before_tree
    );
}

fn assert_forbidden_output(rendered: &str, root: &std::path::Path) {
    for forbidden in [
        root.to_string_lossy().as_ref(),
        "worktree_root",
        "scope_ids",
        "effects",
        "active_lease",
        "active_worktree",
        "claim_effect",
        "leased",
        "WorkerResult",
        "RootActionRequest",
    ] {
        assert!(!rendered.contains(forbidden), "output exposed {forbidden}");
    }
}

#[test]
pub(crate) fn orchestration_refuses_direct_arguments_without_echoing_them() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repository parent");
    let input = "not-a-route-secret";
    let invocation = ParsedInvocation {
        command: SuccessorCommand::Inspect(InspectTarget::Orchestration),
        effect: EffectClass::Read,
        output_mode: OutputMode::Json,
        arguments: vec![OptionArgument {
            name: OptionName::Finding,
            value: ParsedValue::Identifier(input.to_owned()),
        }],
    };

    let streams = execute_invocation(root, invocation).render(OutputMode::Json);

    assert_eq!(streams.exit_code, 2);
    let diagnostic = String::from_utf8(streams.stderr).unwrap();
    assert!(diagnostic.contains("successor_runtime_unexpected_arguments"));
    assert!(!diagnostic.contains(input));
}
