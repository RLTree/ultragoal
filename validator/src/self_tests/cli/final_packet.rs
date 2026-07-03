use crate::cli::final_packet::{FinalPacketCommand, parse, receipt, run};
use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn copy_schema_catalog(root: &Path) {
    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let src = repo.join("schemas");
    let dst = root.join("schemas");
    std::fs::create_dir_all(&dst).expect("schema dst");
    for entry in std::fs::read_dir(src).expect("schemas") {
        let entry = entry.expect("schema entry");
        if entry.path().is_file() {
            std::fs::copy(entry.path(), dst.join(entry.file_name())).expect("copy schema");
        }
    }
}

fn write_manifest(root: &Path) {
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"version":"0.0.0-test"}),
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
}

fn write_final_packet_green_inputs(root: &Path) -> String {
    copy_schema_catalog(root);
    write_manifest(root);
    let current = crate::package::inventory::package_digest(root).expect("digest");
    crate::self_tests::audit::final_packet::receipt_fixtures::write_green_proof(root, &current);
    current
}

#[test]
fn final_packet_receipt_builds_fail_closed_and_green_paths() {
    let blocked =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("final-packet-cli-blocked");
    copy_schema_catalog(&blocked);
    write_manifest(&blocked);
    let missing_source_audit = receipt(&blocked).expect("missing source audit receipt");
    assert_eq!(missing_source_audit["source_audit"]["status"], "fail");
    assert_eq!(missing_source_audit["registry_exposure"]["status"], "fail");
    write_json(
        &blocked.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({"status":"fail"}),
    );
    write_json(
        &blocked.join("validation_artifacts/ultragoal-audit/active-registry-exposure-current.json"),
        &json!({
            "status":"fail",
            "failure":{"blocked_claim_classes":["custom_registry_block"]}
        }),
    );
    let blocked_receipt = blocked.join("validation_artifacts/review/final-packet-proof.json");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: blocked.clone(),
        command: crate::Command::FinalPacket(FinalPacketCommand {
            receipt: blocked_receipt.clone(),
        }),
    })
    .expect("final packet command");
    assert_eq!(code, 1);
    let value = crate::json_boundary::read_json(&blocked_receipt).expect("blocked receipt");
    assert_eq!(value["status"], "fail");
    assert_eq!(value["observability"]["status"], "fail");
    assert_eq!(value["observability"]["operation"], "final-packet.prove");
    assert_eq!(
        value["observability"]["candidate_digest"],
        value["candidate_digest"]
    );
    assert!(
        value["run_id"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
    );
    assert!(
        value["correlation_id"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
    );
    assert!(
        value["why_failed"]
            .as_str()
            .is_some_and(|text| text.contains("final_packet_proof"))
    );
    assert!(
        value["next_repair"]
            .as_str()
            .is_some_and(|text| !text.is_empty() && text != "none")
    );
    assert_eq!(
        value["observability"]["log_stream_digest"],
        crate::digest::canonical_json(&value["observability"]["event"])
    );
    assert_eq!(value["claim_ceiling"], "withheld_or_blocked");
    assert_eq!(value["packet"]["exists"], false);
    assert!(value["packet"]["digest"].is_null());
    assert!(
        value["blocked_claim_classes"]
            .as_array()
            .expect("blocked claims")
            .iter()
            .any(|claim| claim.as_str() == Some("app_registry_or_reviewer_exposure"))
    );
    assert!(
        value["failure"]["observed_failures"]
            .as_array()
            .expect("observed failures")
            .iter()
            .any(|failure| failure
                .as_str()
                .is_some_and(|text| text.contains("final_packet_proof_ref_digest_mismatch"))),
        "{value}"
    );
    assert!(
        value["blocked_claim_classes"]
            .as_array()
            .expect("blocked claims")
            .iter()
            .any(|claim| claim.as_str() == Some("completion"))
    );
    assert!(
        value["blocked_claim_classes"]
            .as_array()
            .expect("blocked claims")
            .iter()
            .any(|claim| claim.as_str() == Some("custom_registry_block"))
    );
    assert!(
        parse(&[
            "packet".to_string(),
            "prove".to_string(),
            "--receipt".to_string(),
            "target/self-tests/packet-proof.json".to_string()
        ])
        .expect("packet alias")
        .is_some()
    );
    let missing_arg = parse(&["final-packet".to_string(), "prove".to_string()])
        .expect_err("missing final-packet receipt path");
    assert!(missing_arg.contains("missing required argument --receipt"));
    std::fs::remove_dir_all(blocked).expect("cleanup blocked final packet root");

    let fail_closed = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "final-packet-cli-fail-closed-source",
    );
    let current = write_final_packet_green_inputs(&fail_closed);
    write_json(
        &fail_closed
            .join("validation_artifacts/ultragoal-audit/active-registry-exposure-current.json"),
        &json!({
            "status": "fail",
            "target_revision": {"kind": "package_digest", "value": current},
            "claim_ceiling": "withheld_or_blocked"
        }),
    );
    let value = receipt(&fail_closed).expect("fail-closed receipt");
    assert_eq!(value["status"], "fail");
    assert_eq!(value["source_audit"]["status"], "pass");
    assert!(value["source_audit"]["self_rewriting_authority"].is_null());
    assert_eq!(
        crate::json_boundary::read_json(
            &fail_closed.join("validation_artifacts/ultragoal-audit/validator-receipt.json")
        )
        .expect("source audit")["status"],
        "pass"
    );
    std::fs::remove_dir_all(fail_closed).expect("cleanup fail-closed final packet root");

    let green =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("final-packet-cli-green");
    let current = write_final_packet_green_inputs(&green);
    let value = receipt(&green).expect("green receipt");
    assert_eq!(value["status"], "pass", "{value}");
    assert_eq!(value["target_revision"]["value"], current);
    assert_eq!(value["claim_ceiling"], "final_packet_evidence_dereferenced");
    assert_eq!(
        value["package_receipts"]
            .as_array()
            .expect("package receipts")
            .len(),
        3
    );

    let output = green.join("validation_artifacts/review/final-packet-proof.json");
    assert_eq!(
        run(
            &green,
            &FinalPacketCommand {
                receipt: output.clone()
            }
        )
        .expect("green run"),
        0
    );
    let written = crate::json_boundary::read_json(&output).expect("written green receipt");
    assert_eq!(written["status"], "pass");
    assert_eq!(written["observability"]["status"], "pass");
    assert_eq!(written["why_failed"], "none");
    let child_spans = written["observability"]["trace"]["child_spans"]
        .as_array()
        .expect("child spans");
    for label in [
        "cli_performance",
        "registry_exposure",
        "source_audit",
        "coverage",
        "package_receipt_0",
        "package_receipt_1",
        "package_receipt_2",
    ] {
        assert!(
            child_spans.iter().any(|span| {
                span["span_kind"] == "receipt_deref"
                    && span["dereferenced_receipt_label"] == label
                    && span["parent_span_id"] == written["observability"]["trace"]["span_id"]
            }),
            "missing final-packet dereference span {label}: {child_spans:?}"
        );
    }
    assert_eq!(
        written["observability"]["supported_claims"][0],
        "final_packet_evidence_dereferenced"
    );
    std::fs::remove_dir_all(green).expect("cleanup green final packet root");
}
