use crate::audit::contract::SemanticClass;
use crate::semantic::receipt::{GenerateOptions, build_receipt};
use serde_json::json;
use std::path::PathBuf;

fn options(kind: &str) -> GenerateOptions {
    GenerateOptions {
        root: PathBuf::from("."),
        input: PathBuf::from("fixture.json"),
        out_dir: PathBuf::from("out"),
        implementation_kind: kind.to_string(),
        provider: Some("fake-provider".to_string()),
        model: Some("fake-model".to_string()),
        contract_id: "ultragoal-semantic-classifier".to_string(),
        contract_version: "v1".to_string(),
        prompt_contract_digest: Some(crate::digest::ZERO.to_string()),
        producer_actor_id: "producer".to_string(),
        classifier_actor_id: "classifier".to_string(),
    }
}

#[test]
fn generator_refuses_model_receipts_without_external_model_output() {
    let claim = json!({
        "id": "CLAIM-001",
        "title": "Engine batch processor completes internal CLI work",
        "description": "Headless CLI batch processing."
    });
    let result = build_receipt(&claim, &options("model"));
    assert!(result.is_err());
}

#[test]
fn generator_refuses_human_reviewer_receipts_without_attestation() {
    let claim = json!({
        "id": "CLAIM-001",
        "title": "Engine batch processor completes internal CLI work",
        "description": "Headless CLI batch processing."
    });
    let result = build_receipt(&claim, &options("human_reviewer"));
    assert!(result.is_err());
}

#[test]
fn classifier_keeps_engine_interface_language_runtime_only() {
    for title in [
        "API interface parser validates schema",
        "client library parser completes engine task",
        "database view materialization completes backend job",
        "form schema validation completes CLI command",
    ] {
        let claim = json!({
            "id": "CLAIM-001",
            "title": title,
            "description": "Engine-only runtime work with no user-facing product surface.",
            "product_applicability": {
                "user_facing": false,
                "product_surface": false,
                "ui_or_control_surface": false
            }
        });
        let classes = crate::semantic::receipt::classifier::classify(&claim, &title.to_lowercase());
        assert_eq!(
            classes,
            vec![SemanticClass::RuntimeCliBackendOnlyEngineOnly]
        );
    }
}

#[test]
fn classifier_still_flags_concrete_desktop_client_language() {
    let claim = json!({"id": "CLAIM-001", "title": "Desktop client inspects runs"});
    let classes =
        crate::semantic::receipt::classifier::classify(&claim, "desktop client inspects runs");
    assert!(classes.contains(&SemanticClass::LocalAppDesktopBrowser));
}

#[test]
fn classifier_maps_visible_product_surfaces_to_required_proof_gates() {
    let claim = json!({
        "id":"CLAIM-SURFACES",
        "product_applicability":{"user_facing":true}
    });
    let classes = crate::semantic::receipt::classifier::classify(
        &claim,
        "The run console and workflow launcher make the app visible in a control panel.",
    );
    for expected in [
        SemanticClass::RunConsole,
        SemanticClass::WorkflowLauncher,
        SemanticClass::LocalAppDesktopBrowser,
        SemanticClass::UiControlSurface,
        SemanticClass::ProductUserFacingSurface,
    ] {
        assert!(classes.contains(&expected), "{expected:?}: {classes:?}");
    }
    let gates = crate::semantic::receipt::classifier::proof_gates(&classes);
    assert!(gates.contains(&crate::audit::contract::ProofGate::ProductCohesionReceipt));
    assert!(gates.contains(&crate::audit::contract::ProofGate::UiJourneyEvidence));
    assert!(gates.contains(&crate::audit::contract::ProofGate::AccessibilityEvidence));

    let backend_app = crate::semantic::receipt::classifier::classify(
        &json!({"id":"BACKEND"}),
        "The backend cli engine app works in headless server mode.",
    );
    assert_eq!(
        backend_app,
        vec![SemanticClass::RuntimeCliBackendOnlyEngineOnly]
    );

    let install_publication = crate::semantic::receipt::classifier::proof_gates(&[
        SemanticClass::InstallVisibleSelectableActiveInCodex,
        SemanticClass::PublicationMarketplaceCatalogWorkspaceRegistry,
        SemanticClass::AmbiguousNeedsReviewerClassification,
    ]);
    assert!(
        install_publication.contains(&crate::audit::contract::ProofGate::InstallVisibilityReceipt)
    );
    assert!(
        install_publication
            .contains(&crate::audit::contract::ProofGate::PublicationExternalAttestation)
    );
    assert!(
        install_publication.contains(&crate::audit::contract::ProofGate::ReviewerClassification)
    );
}

#[test]
fn generator_handles_absolute_inputs_missing_claims_unknown_kind_and_sanitized_outputs() {
    let root = crate::self_tests::boundaries::support::temp_root("semantic-generator");
    std::fs::create_dir_all(&root).expect("root");
    let input = root.join("claims.json");
    std::fs::write(&input, "{}").expect("missing claims manifest");
    let mut opts = options("deterministic_backstop");
    opts.root = root.clone();
    opts.input = input.clone();
    opts.out_dir = root.join("out");
    assert!(
        crate::semantic::receipt::generate(opts)
            .expect_err("missing claims rejected")
            .contains("claims must be a list")
    );

    std::fs::write(
        &input,
        serde_json::to_vec(&json!({"claims":[{
            "id":"CLAIM/WEIRD VALUE",
            "title":"Review ready",
            "description":"Live browser claim."
        }]}))
        .expect("claims json"),
    )
    .expect("write claims");
    let mut opts = options("deterministic_backstop");
    opts.root = root.clone();
    opts.input = input.clone();
    opts.out_dir = root.join("out");
    opts.producer_actor_id = "same".to_string();
    opts.classifier_actor_id = "same".to_string();
    crate::semantic::receipt::generate(opts).expect("generate semantic receipt");
    let receipt = crate::json_boundary::read_json(
        &root.join("out/CLAIM-WEIRD-VALUE.semantic-classification-receipt.json"),
    )
    .expect("semantic receipt");
    assert_eq!(receipt["claim_id"], "CLAIM/WEIRD VALUE");
    assert_eq!(receipt["actor_disjoint"], false);

    let claim = json!({"id":"CLAIM-UNKNOWN","title":"Unknown classifier kind"});
    assert!(
        build_receipt(&claim, &options("llm_oracle"))
            .expect_err("unknown implementation kind")
            .contains("unknown classifier implementation kind")
    );

    let mut opts = options("deterministic_backstop");
    opts.root = root.clone();
    opts.input = input.clone();
    opts.out_dir = input.clone();
    assert!(
        crate::semantic::receipt::generate(opts)
            .expect_err("file cannot become output directory")
            .contains("create receipt dir failed")
    );

    let blocking_out = root.join("blocking-out");
    std::fs::create_dir_all(&blocking_out).expect("blocking out dir");
    std::fs::create_dir_all(blocking_out.join("CLAIM.semantic-classification-receipt.json"))
        .expect("blocking receipt path");
    std::fs::write(
        &input,
        serde_json::to_vec(&json!({"claims":[{"id":"CLAIM","title":"CLI claim"}]}))
            .expect("blocking input json"),
    )
    .expect("write blocking input");
    let mut opts = options("deterministic_backstop");
    opts.root = root.clone();
    opts.input = input;
    opts.out_dir = blocking_out;
    assert!(
        crate::semantic::receipt::generate(opts)
            .expect_err("directory receipt path rejected")
            .contains("Is a directory")
    );
    std::fs::remove_dir_all(root).expect("cleanup semantic generator");
}
