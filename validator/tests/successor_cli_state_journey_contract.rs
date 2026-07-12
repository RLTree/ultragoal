#![cfg(unix)]

#[path = "successor_cli_state_journey_contract/assertions.rs"]
mod assertions;
#[path = "successor_cli_state_journey_contract/fixture.rs"]
mod fixture;
#[path = "successor_cli_state_journey_contract/snapshot.rs"]
mod snapshot;

use assertions::{assert_diagnostic, assert_payload, machine_value};
use fixture::{JourneyRepository, journey_cases};
use serde_json::Value;
use snapshot::observe;

const PRIVATE_CANARY: &str = "SUCCESSOR_READ_PRIVATE_CANARY_047";

#[test]
fn fresh_binary_classifies_clean_dirty_and_conflicting_authority_without_writes() {
    let cases = journey_cases();
    assert_eq!(
        cases
            .iter()
            .map(|case| case.id.as_str())
            .collect::<Vec<_>>(),
        ["clean", "dirty", "conflicting-authority"]
    );

    for case in cases {
        let repository = JourneyRepository::new(&case);
        let before = observe(repository.root());
        assert_eq!(
            !before.status.is_empty(),
            case.dirty,
            "fixture Git status disagrees with {}",
            case.id
        );

        let context = repository.run(&["--json", "inspect", "context"]);
        let context_value = assert_payload(&context, 0, "HarnessPublicContext-v1", PRIVATE_CANARY);
        assert_eq!(
            context_value["candidate"]["dirty"], case.dirty,
            "{}",
            case.id
        );
        assert_eq!(
            observe(repository.root()),
            before,
            "{} wrote during context read",
            case.id
        );

        let findings = repository.run(&["--json", "inspect", "findings"]);
        let findings_value = assert_payload_any_exit(
            &findings,
            &[0, 1],
            "ProductStateFindings-v1",
            PRIVATE_CANARY,
        );
        if let Some(expected) = &case.expected_finding_code {
            assert!(
                findings_value["findings"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|row| row["code"] == expected.as_str())),
                "{} did not expose expected finding {expected}: {findings_value}",
                case.id
            );
        }
        assert_eq!(
            observe(repository.root()),
            before,
            "{} wrote during state read",
            case.id
        );
    }
}

#[test]
fn help_parse_inspect_next_diagnose_and_query_are_recursively_zero_write() {
    let case = journey_cases()
        .into_iter()
        .find(|case| case.id == "dirty")
        .expect("dirty case");
    let repository = JourneyRepository::new(&case);
    let initial = observe(repository.root());

    let payloads: &[(&[&str], &[i32], &str)] = &[
        (&["--json", "--help"], &[0], "harness-ultragoal.cli-help.v1"),
        (
            &["--json", "--version"],
            &[0],
            "harness-ultragoal.cli-version.v1",
        ),
        (
            &["--json", "inspect", "context"],
            &[0],
            "HarnessPublicContext-v1",
        ),
        (
            &["--json", "inspect", "capabilities"],
            &[0],
            "HarnessCapabilities-v1",
        ),
        (
            &["--json", "inspect", "inventory"],
            &[0, 1],
            "AuthorityCatalog-v1",
        ),
        (&["--json", "inspect"], &[0, 1], "ProductStateSummary-v1"),
        (
            &["--json", "inspect", "findings"],
            &[0, 1],
            "ProductStateFindings-v1",
        ),
        (
            &["--json", "inspect", "claims"],
            &[0, 1],
            "ProductStateClaims-v1",
        ),
        (&["--json", "next"], &[0, 1, 3], "ProductStateNext-v1"),
        (&["--json", "diagnose"], &[0, 1], "ProductStateDiagnose-v1"),
        (
            &["--json", "observe", "query"],
            &[0],
            "ObservabilityQuery-v1",
        ),
    ];
    for (args, exits, schema) in payloads {
        let before = observe(repository.root());
        let output = repository.run(args);
        assert_payload_any_exit(&output, exits, schema, PRIVATE_CANARY);
        assert_eq!(observe(repository.root()), before, "hidden write: {args:?}");
    }

    for args in [
        &["--json", PRIVATE_CANARY][..],
        &["--json", "inspect", "--unknown-private-option"][..],
    ] {
        let before = observe(repository.root());
        let output = repository.run(args);
        assert_diagnostic(&output, 2, "harness-ultragoal.cli-error.v1", PRIVATE_CANARY);
        assert_eq!(
            observe(repository.root()),
            before,
            "parse path wrote: {args:?}"
        );
    }

    let before = observe(repository.root());
    let missing_root = repository.run_raw(&["--json", "--root"]);
    assert_diagnostic(
        &missing_root,
        2,
        "harness-ultragoal.cli-error.v1",
        PRIVATE_CANARY,
    );
    assert_eq!(
        observe(repository.root()),
        before,
        "root parse failure wrote"
    );
    assert_eq!(observe(repository.root()), initial);
}

#[test]
fn diagnosis_repair_and_next_are_stable_across_repeat_reads() {
    let case = journey_cases()
        .into_iter()
        .find(|case| case.id == "conflicting-authority")
        .expect("conflict case");
    let repository = JourneyRepository::new(&case);
    let before = observe(repository.root());

    let findings_output = repository.run(&["--json", "inspect", "findings"]);
    let findings = assert_payload_any_exit(
        &findings_output,
        &[1],
        "ProductStateFindings-v1",
        PRIVATE_CANARY,
    );
    let selected = findings["findings"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["code"] == "parallel_authority"))
        .expect("parallel authority finding");
    let finding_id = selected["finding_id"].as_str().expect("finding id");
    let repair_id = selected["repair"]["repair_id"].as_str().expect("repair id");

    let diagnosis_output = repository.run(&["--json", "diagnose", "--finding", finding_id]);
    let diagnosis = assert_payload(
        &diagnosis_output,
        1,
        "ProductStateDiagnose-v1",
        PRIVATE_CANARY,
    );
    assert_eq!(diagnosis["findings"].as_array().map(Vec::len), Some(1));
    assert_eq!(diagnosis["repairs"][0]["repair_id"], repair_id);
    assert_eq!(diagnosis["repairs"][0]["effect"], "read");
    assert_eq!(diagnosis["repairs"][0]["authority"], "root");
    assert_eq!(diagnosis["repairs"][0]["rerun_command_id"], "inspect-json");
    assert_eq!(diagnosis["observability"]["claim_effect"], "none");

    let next_one = repository.run(&["--json", "next"]);
    let next_two = repository.run(&["--json", "next"]);
    let next_one_value =
        assert_payload_any_exit(&next_one, &[1, 3], "ProductStateNext-v1", PRIVATE_CANARY);
    let next_two_value =
        assert_payload_any_exit(&next_two, &[1, 3], "ProductStateNext-v1", PRIVATE_CANARY);
    assert_eq!(
        next_one.stdout, next_two.stdout,
        "repeat next output drifted"
    );
    assert_eq!(next_one_value, next_two_value);
    assert_eq!(diagnosis["state_id"], next_one_value["state_id"]);
    assert_eq!(diagnosis["next_action"], next_one_value["next_action"]);
    assert_eq!(
        diagnosis["claim_ceilings"],
        next_one_value["claim_ceilings"]
    );
    assert_eq!(
        observe(repository.root()),
        before,
        "repeat reads mutated repository"
    );
}

#[test]
fn machine_streams_are_separate_non_echoing_and_fail_closed() {
    let case = journey_cases().remove(0);
    let repository = JourneyRepository::new(&case);
    let before = observe(repository.root());

    let success = repository.run(&["--json", "inspect", "context"]);
    assert_payload(&success, 0, "HarnessPublicContext-v1", PRIVATE_CANARY);

    let actionable = repository.run(&["--json", "diagnose"]);
    assert_payload(&actionable, 1, "ProductStateDiagnose-v1", PRIVATE_CANARY);

    let invalid = repository.run(&["--json", PRIVATE_CANARY]);
    assert_diagnostic(
        &invalid,
        2,
        "harness-ultragoal.cli-error.v1",
        PRIVATE_CANARY,
    );

    let unsupported = repository.run(&[
        "--json",
        "package",
        "build",
        "--output",
        "private-output-canary.json",
    ]);
    let unsupported_value = assert_diagnostic(
        &unsupported,
        4,
        "HarnessDiagnostic-v1",
        "private-output-canary.json",
    );
    assert_eq!(unsupported_value["exit_class"], "unsupported_capability");
    assert_eq!(observe(repository.root()), before);
}

fn assert_payload_any_exit(
    output: &std::process::Output,
    exits: &[i32],
    schema: &str,
    private: &str,
) -> Value {
    assert!(
        exits.contains(&output.status.code().unwrap_or(-1)),
        "unexpected exit: {output:?}"
    );
    let value = machine_value(&output.stdout, schema, private);
    assert!(
        output.stderr.is_empty(),
        "payload contaminated stderr: {output:?}"
    );
    value
}
