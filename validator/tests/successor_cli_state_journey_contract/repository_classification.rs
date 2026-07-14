use super::PRIVATE_CANARY;
use super::assertions::{assert_diagnostic, assert_payload, assert_payload_any_exit};
use super::fixture::{JourneyRepository, journey_cases};
use super::snapshot::observe;

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
