use super::*;

pub(crate) fn selected_finding(repository: &JourneyRepository) -> SelectedFinding {
    let value = assert_payload_repeat_zero_write(
        repository,
        &["--json", "inspect", "findings"],
        &[1],
        "ProductStateFindings-v1",
    );
    let row = value["findings"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["code"] == "parallel_authority"))
        .expect("parallel authority finding");
    SelectedFinding {
        finding_id: row["finding_id"].as_str().expect("finding id").to_owned(),
        repair_id: row["repair"]["repair_id"]
            .as_str()
            .expect("repair id")
            .to_owned(),
    }
}

pub(crate) fn open_store(repository: &JourneyRepository, binding: &Binding) -> EventStore {
    fs::create_dir_all(
        repository
            .store_path()
            .parent()
            .expect("observability spool parent"),
    )
    .expect("create observability spool");
    EventStore::open_bound(
        repository.store_path(),
        &binding.context_id,
        &binding.candidate_id,
        &binding.source_id,
    )
    .expect("open candidate-bound event store")
}

pub(crate) fn event(binding: &Binding, id: &str, sequence: u64, operation: &str) -> SemanticEvent {
    SemanticEvent::new(
        &binding.context_id,
        &binding.candidate_id,
        &binding.source_id,
        id,
        sequence,
        sequence,
        operation,
        "fail",
    )
    .expect("candidate-bound semantic event")
}

pub(crate) fn query(binding: &Binding) -> EventQuery {
    EventQuery::new(
        &binding.context_id,
        &binding.candidate_id,
        &binding.source_id,
    )
    .expect("candidate-bound query")
}

pub(crate) fn assert_no_connection(listener: &TcpListener) {
    listener
        .set_nonblocking(true)
        .expect("nonblocking network canary");
    match listener.accept() {
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
        Ok((_, address)) => panic!("unexpected disabled-export connection from {address}"),
        Err(error) => panic!("network canary failed: {error}"),
    }
}

pub(crate) fn safe_label(label: &str) -> String {
    label
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect()
}

#[test]
pub(crate) fn fixture_catalog_closes_the_required_behavior_classes() {
    let catalog = catalog();
    assert_eq!(
        catalog.schema_version,
        "ObservabilityLocalDiagnosisFixtures-v1"
    );
    assert_eq!(catalog.temporary_root, BASE);
    assert_eq!(catalog.source_id, SOURCE_ID);
    assert_eq!(catalog.claim_effect, "none");
    assert_eq!(
        catalog.external_export_default,
        "disabled-safe-default-OD-004-OD-007"
    );
    let ids = catalog
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), catalog.cases.len(), "duplicate fixture id");
    let classes = catalog
        .cases
        .iter()
        .map(|case| case.class.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        classes,
        [
            "false-pass",
            "mutation",
            "negative",
            "network",
            "positive",
            "privacy",
            "race",
            "read",
            "security",
            "stale-candidate",
            "unknown-row",
        ]
        .into_iter()
        .collect()
    );
    assert!(catalog.cases.iter().all(|case| !case.expect.is_empty()));
}

#[test]
pub(crate) fn fresh_binary_absent_help_query_diagnose_and_export_refusal_are_zero_write() {
    let repository = JourneyRepository::new("absent-read", true, true);
    let initial = observe(repository.root());
    assert!(!initial.git_status.is_empty(), "fixture must remain dirty");

    assert_payload_repeat_zero_write(
        &repository,
        &["--json", "--help"],
        &[0],
        "harness-ultragoal.cli-help.v1",
    );
    let binding = public_binding(&repository);
    assert_eq!(binding.source_id, SOURCE_ID);
    let finding = selected_finding(&repository);
    let diagnosis = assert_payload_repeat_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(diagnosis["observability"]["store_status"], "absent");
    assert_eq!(
        diagnosis["observability"]["explanation"]["classification"],
        "missing-evidence"
    );
    assert_eq!(diagnosis["claim_effect"], "none");

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind network canary");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let environment = [
        ("HTTP_PROXY", endpoint.as_str()),
        ("HTTPS_PROXY", endpoint.as_str()),
        ("ALL_PROXY", endpoint.as_str()),
        ("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint.as_str()),
        ("ULTRAGOAL_OBSERVABILITY_EXPORT_ENDPOINT", endpoint.as_str()),
    ];
    let before_export = observe(repository.root());
    let export = repository.run_with_env(
        &[
            "--json",
            "observe",
            "export",
            "--output",
            "journey-export.json",
            "--approve-export",
        ],
        &environment,
    );
    assert_eq!(export.status.code(), Some(3), "{export:?}");
    assert!(export.stdout.is_empty(), "{export:?}");
    assert_eq!(
        machine(&export.stderr)["diagnostic_id"],
        "successor_runtime_authority_required"
    );
    assert_private_absent(&export, repository.root());
    assert_eq!(observe(repository.root()), before_export);
    assert!(!repository.root().join("journey-export.json").exists());
    assert_no_connection(&listener);
    assert_eq!(observe(repository.root()), initial);
}
