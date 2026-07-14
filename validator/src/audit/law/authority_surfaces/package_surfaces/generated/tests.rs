use serde_json::json;
use std::fs;
use std::path::PathBuf;

const OUTPUT: &str = "docs/generated/observability/command-inventory.json";
const REGISTRY: &str = "migration/generated-surface-authority.json";

fn root(label: &str, bytes: &[u8]) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    fs::create_dir_all(root.join("docs/generated/observability")).expect("generated directory");
    fs::create_dir_all(root.join("migration")).expect("migration directory");
    fs::write(root.join(OUTPUT), bytes).expect("output");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources": [OUTPUT]}),
    )
    .expect("manifest");
    crate::json_boundary::write_json(
        &root.join(REGISTRY),
        &json!({
            "schema_version": "GeneratedSurfaceAuthority-v2",
            "contract_id": "harness-ultragoal-successor-contract-v2",
            "surfaces": [{
                "disposition": "retained_context",
                "output": OUTPUT,
                "sha256": crate::digest::bytes(bytes).trim_start_matches("sha256:"),
                "reason": "preserved predecessor context",
                "replacement_targets": ["HCT-OBSERVE"],
                "preserve": true,
                "physical_deletion_authorized": false
            }]
        }),
    )
    .expect("registry");
    root
}

fn output_row(value: &serde_json::Value) -> &serde_json::Value {
    value["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["path_or_symbol"] == OUTPUT)
        .expect("output row")
}

#[test]
fn retained_context_is_never_projected_as_generated_authority() {
    let root = root("package-surface-retained-context", b"SECRET_CANARY");
    let value = crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    let row = output_row(&value);
    assert_eq!(row["surface_kind"], "retained_context");
    assert_eq!(row["authority_level"], "retained_context_no_claim");
    assert_eq!(row["claim_surfaces_allowed"], json!([]));
    assert_ne!(row["proof_surface"], "generated_projection");
    assert_ne!(row["canonical_owner"], OUTPUT);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn tampered_retained_context_stays_non_authoritative_and_fails() {
    let root = root("package-surface-retained-tamper", b"retained");
    fs::write(root.join(OUTPUT), b"tampered").expect("tamper");
    let value = crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    let row = output_row(&value);
    assert_eq!(row["authority_level"], "invalid_generated_authority");
    assert_eq!(row["claim_surfaces_allowed"], json!([]));
    let inventory = [OUTPUT.to_string()].into_iter().collect();
    let failures =
        crate::audit::law::authority_surfaces::package_surface_failures_for_test(&root, &inventory);
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure
                .contains("failure_class=generated_disposition_batch_invalid")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("invalid_generated_path_count=1")),
        "{failures:?}"
    );
    let failure_document = serde_json::to_string(&failures).expect("failure document");
    assert!(!failure_document.contains(OUTPUT), "{failure_document}");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn exported_generated_failures_never_echo_attacker_controlled_safe_paths() {
    const CANARIES: &[&str] = &[
        "docs/generated/SECRET_CANARY.json",
        "docs/generated/nested/SECRET_CANARY_file-1.json",
        "examples/generated/SECRET-CANARY.json",
        "generated/secret_canary.json",
    ];
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "package-surface-generated-failure-non-echo",
    );
    fs::create_dir_all(root.join("migration")).expect("migration directory");
    for path in CANARIES {
        let full = root.join(path);
        fs::create_dir_all(full.parent().expect("generated parent")).expect("generated directory");
        fs::write(full, b"attacker controlled context").expect("generated context");
    }
    crate::json_boundary::write_json(
        &root.join(REGISTRY),
        &json!({
            "schema_version": "GeneratedSurfaceAuthority-v2",
            "contract_id": "harness-ultragoal-successor-contract-v2",
            "surfaces": []
        }),
    )
    .expect("empty registry");
    let inventory = CANARIES.iter().map(|path| (*path).to_string()).collect();

    let inner = super::State::build(&root, &inventory).failures();
    assert_eq!(inner.len(), 1, "{inner:?}");
    assert_eq!(inner[0].0, "generated-disposition-batch");
    assert!(inner[0].1.contains("invalid_generated_path_count=4"));
    let exported =
        crate::audit::law::authority_surfaces::package_surface_failures_for_test(&root, &inventory);
    let document =
        serde_json::to_string(&json!({"failures": exported})).expect("exported failure document");
    assert!(
        document.contains("failure_class=generated_disposition_batch_invalid"),
        "{document}"
    );
    for canary in CANARIES {
        assert!(!document.contains(canary), "path escaped: {document}");
    }
    for token in ["SECRET_CANARY", "SECRET-CANARY", "secret_canary"] {
        assert!(!document.contains(token), "token escaped: {document}");
    }
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn live_retained_registry_rows_are_all_no_claim_package_context() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let registry = crate::json_boundary::read_json(&root.join(REGISTRY)).expect("live registry");
    let outputs = registry["surfaces"]
        .as_array()
        .expect("live registry surfaces")
        .iter()
        .map(|surface| {
            assert_eq!(surface["disposition"], "retained_context");
            surface["output"]
                .as_str()
                .expect("retained output")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(outputs.len(), 9, "live retained row count drifted");

    let value = crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    for output in outputs {
        let row = value["rows"]
            .as_array()
            .expect("package surface rows")
            .iter()
            .find(|row| row["path_or_symbol"] == output)
            .unwrap_or_else(|| panic!("missing live retained row: {output}"));
        assert_eq!(row["surface_kind"], "retained_context", "{output}");
        assert_eq!(
            row["authority_level"], "retained_context_no_claim",
            "{output}"
        );
        assert_eq!(
            row["proof_surface"], "retained_context_no_claim",
            "{output}"
        );
        assert_eq!(row["claim_surfaces_allowed"], json!([]), "{output}");
    }
}
