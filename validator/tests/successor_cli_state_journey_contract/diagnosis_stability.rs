use super::PRIVATE_CANARY;
use super::assertions::{assert_diagnostic, assert_payload, assert_payload_any_exit};
use super::fixture::{JourneyRepository, journey_cases};
use super::snapshot::observe;

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
