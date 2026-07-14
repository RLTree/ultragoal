use super::*;

pub(crate) fn scratch_snapshot(
    root: &std::path::Path,
) -> Vec<(std::path::PathBuf, String, Vec<u8>)> {
    fn visit(
        root: &std::path::Path,
        path: &std::path::Path,
        rows: &mut Vec<(std::path::PathBuf, String, Vec<u8>)>,
    ) {
        let metadata = std::fs::symlink_metadata(path).expect("scratch metadata");
        let relative = path
            .strip_prefix(root)
            .expect("scratch relative")
            .to_owned();
        if metadata.file_type().is_symlink() {
            rows.push((
                relative,
                "symlink".to_owned(),
                std::fs::read_link(path)
                    .expect("scratch symlink target")
                    .as_os_str()
                    .as_encoded_bytes()
                    .to_vec(),
            ));
        } else if metadata.is_dir() {
            rows.push((relative, "directory".to_owned(), Vec::new()));
            let mut children = std::fs::read_dir(path)
                .expect("scratch read dir")
                .map(|entry| entry.expect("scratch entry").path())
                .collect::<Vec<_>>();
            children.sort();
            for child in children {
                visit(root, &child, rows);
            }
        } else if metadata.is_file() {
            rows.push((
                relative,
                "file".to_owned(),
                std::fs::read(path).expect("scratch file"),
            ));
        } else {
            rows.push((relative, "special".to_owned(), Vec::new()));
        }
    }

    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}

pub(crate) const CLAIMS: [&str; 5] = [
    "CL-PACKAGE",
    "CL-INSTALL",
    "CL-ORCHESTRATION",
    "CL-RELEASE",
    "CL-COMPLETION",
];

pub(crate) fn false_pass_position(observations: &[Observation]) -> usize {
    observations
        .iter()
        .position(|item| item.envelope().obligation.kind == ObligationKind::FalsePassControl)
        .expect("false-pass control")
}

pub(crate) fn decide(
    ledger: &mut DecisionLedger,
    definitions: &ClaimDefinitions,
    claim_id: &str,
    observations: Vec<Observation>,
) -> super::super::claims::ClaimDecision {
    let ids = submit(ledger, observations);
    ledger.decide(
        definitions,
        claim_id,
        context_id(),
        candidate_id(),
        now(),
        &reviewer(definitions, claim_id),
        &ids,
    )
}

#[test]
pub(crate) fn sealed_semantic_model_binds_exact_adopted_control_names_without_process_proof() {
    let definitions = definitions();
    for claim_id in CLAIMS {
        let observations = observations_for(&definitions, claim_id, "authority-positive");
        for observation in observations
            .iter()
            .filter(|item| item.envelope().obligation.kind == ObligationKind::FalsePassControl)
        {
            let envelope = observation.envelope();
            let observed = envelope
                .false_pass_model
                .as_ref()
                .expect("semantic control model");
            let model = observed.model();
            assert!(observed.verify_integrity().is_ok());
            assert_eq!(envelope.result.result_digest(), observed.observed_digest());
            assert_eq!(model.registry_digest(), definitions.registry_digest());
            assert_eq!(model.claim_id(), claim_id);
            assert_eq!(model.control_id(), envelope.obligation.id);
            assert!(
                model
                    .expected_failure_contract()
                    .starts_with("claims-negative-control-rejected:")
            );
            assert!(model.control_definition_digest().starts_with("sha256:"));
            assert!(model.model_spec_digest().starts_with("sha256:"));
            assert!(model.authority_nonce().starts_with("sha256:"));
            assert!(model.model_implementation_digest().starts_with("sha256:"));
            assert!(model.modeled_result_digest().starts_with("sha256:"));
            assert!(model.model_record_digest().starts_with("sha256:"));
            assert_ne!(model.modeler().actor_id, observed.observer().actor_id);
            assert_ne!(model.modeler().actor_id, envelope.producer.actor_id);
            assert_ne!(observed.observer().actor_id, envelope.producer.actor_id);
            assert!(model.modeled_at_unix_ms() <= observed.observed_at_unix_ms());
        }
        let mut ledger = super::super::scenario::semantic_model_ledger();
        pass_before(
            &mut ledger,
            &definitions,
            claim_id,
            "authority-positive-prior",
        );
        assert_eq!(
            decide(&mut ledger, &definitions, claim_id, observations).status,
            DecisionStatus::Passed,
            "{claim_id}"
        );
    }
}

#[test]
pub(crate) fn default_product_ledger_rejects_a_well_formed_semantic_model_as_non_executed_proof() {
    let definitions = definitions();
    let claim_id = "CL-SOURCE";
    let mut ledger = DecisionLedger::default();
    let ids = submit(
        &mut ledger,
        observations_for(&definitions, claim_id, "default-ledger-refusal"),
    );
    let decision = ledger.decide(
        &definitions,
        claim_id,
        context_id(),
        candidate_id(),
        now(),
        &reviewer(&definitions, claim_id),
        &ids,
    );
    assert_eq!(decision.status, DecisionStatus::Rejected);
    assert!(
        decision
            .reasons
            .iter()
            .any(|reason| { reason == "claims-false-pass-semantic-model-not-executed-proof" })
    );
}
