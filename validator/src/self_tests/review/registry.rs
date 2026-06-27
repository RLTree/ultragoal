use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn errors(out: &[crate::review::round::ReviewFailure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

#[test]
fn live_registry_exposure_rejects_stale_malformed_and_shape_substitutes() {
    let root = crate::self_tests::boundaries::support::temp_root("review-registry-branches");
    std::fs::create_dir_all(root.join("validation_artifacts")).expect("registry dir");
    let exposure_path = root.join("validation_artifacts/exposure.json");
    let agent_types = crate::review::round::config::PERSONAS
        .iter()
        .map(|spec| {
            json!({
                "agent_type": spec.agent_type,
                "persona": "wrong-persona",
                "custom_agent_path": "wrong.toml",
                "exposed": true,
                "disk_cache_synced": false,
                "global_toml_present": false
            })
        })
        .collect::<Vec<_>>();
    write_json(
        &exposure_path,
        &json!({
            "schema": "harness-ultragoal.multi-agent-registry-exposure.v1",
            "source": "disk-cache-snapshot",
            "captured_at": "2026-06-25T00:00:01Z",
            "session_id": "",
            "round_id": "other-round",
            "agent_types": agent_types
        }),
    );
    let digest = crate::digest::file(&exposure_path).expect("digest");
    let mut out = Vec::new();
    crate::review::round::registry::exposure_errors(
        &root,
        &json!({
            "generated_at": "2026-06-25T00:00:00Z",
            "round_id": "round-1",
            "live_registry_exposure": {"path":"validation_artifacts/exposure.json","digest":digest},
            "reviewers": [{"persona":"contract_claim_falsifier","live_spawn_receipt":{"source_thread_id":"other-session"}}]
        }),
        &mut out,
    );
    let got = errors(&out);
    assert!(got.contains(&"review_round_live_registry_stale"), "{got:?}");
    assert!(
        got.contains(&"review_round_live_registry_agent_mismatch"),
        "{got:?}"
    );
    assert!(
        got.contains(&"review_round_live_registry_disk_sync_missing"),
        "{got:?}"
    );

    out.clear();
    crate::review::round::registry::row_agent_type_error(
        &json!({"agent_type":"wrong"}),
        "contract_claim_falsifier",
        &mut out,
    );
    assert_eq!(out[0].error, "review_round_live_registry_agent_mismatch");

    out.clear();
    crate::review::round::registry::exposure_errors(
        &root,
        &json!({"live_registry_exposure":{"path":"validation_artifacts/exposure.json","digest":crate::self_tests::boundaries::support::sha('0')}}),
        &mut out,
    );
    assert_eq!(out[0].error, "review_round_live_registry_artifact_mismatch");

    std::fs::write(&exposure_path, "{").expect("malformed exposure");
    let digest = crate::digest::file(&exposure_path).expect("malformed digest");
    out.clear();
    crate::review::round::registry::exposure_errors(
        &root,
        &json!({"live_registry_exposure":{"path":"validation_artifacts/exposure.json","digest":digest}}),
        &mut out,
    );
    assert_eq!(
        out[0].error,
        "review_round_live_registry_artifact_malformed"
    );
    std::fs::remove_dir_all(root).expect("cleanup registry branches");
}
