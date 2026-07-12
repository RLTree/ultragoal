use serde_json::{Value, json};
use std::path::Path;

pub(crate) fn sync_persona_refs(root: &Path, row: &mut Value) {
    let Some(role) = row.get("role").and_then(Value::as_str) else {
        return;
    };
    let Some(spec) = crate::review::round::config::review_role_spec(role) else {
        return;
    };
    let manifest_path = spec.agent_manifest_path;
    let Some(manifest_digest) = digest(root, manifest_path) else {
        if let Some(object) = row.as_object_mut() {
            object.remove("agent_manifest_path");
            object.remove("agent_manifest_digest");
        }
        return;
    };
    row["agent_manifest_path"] = json!(manifest_path);
    row["agent_manifest_digest"] = json!(manifest_digest);
    for old in [
        "persona_prompt_path",
        "persona_prompt_digest",
        "custom_agent_path",
        "custom_agent_digest",
    ] {
        row.as_object_mut().expect("reviewer row").remove(old);
    }
    for key in ["proof_anchors_checked", "evidence_artifacts_checked"] {
        retain_existing_refs(root, row, key);
        for path in [manifest_path, spec.focus_path] {
            upsert_ref(root, row, key, path);
        }
    }
}

pub(crate) fn bind_product_fitness(root: &Path, row: &mut Value) {
    if row.get("role").and_then(Value::as_str) != Some("product-journey-reviewer") {
        return;
    }
    let path = "validation_artifacts/harness/product-fitness-receipt.json";
    let receipt = crate::json_boundary::read_json(&root.join(path)).expect("product fitness");
    let generated_at = receipt
        .get("generated_at")
        .and_then(Value::as_str)
        .expect("product fitness generated_at");
    let got = digest(root, path);
    row["product_fitness_receipt_digest"] = json!(got);
    row["product_fitness_disposition"]["receipt"] = json!({"path": path, "digest": got});
    row["product_fitness_disposition"]["receipt_generated_at"] = json!(generated_at);
    row["substitution_rejections_reviewed"] = json!(
        crate::review::round::product::fitness::criteria::required_substitutions()
            .into_iter()
            .collect::<Vec<_>>()
    );
    for key in ["proof_anchors_checked", "evidence_artifacts_checked"] {
        upsert_ref(root, row, key, path);
    }
}

fn retain_existing_refs(root: &Path, row: &mut Value, key: &str) {
    row[key]
        .as_array_mut()
        .expect("artifact refs")
        .retain(|item| {
            item.get("path")
                .and_then(Value::as_str)
                .is_some_and(|path| !legacy_agent_ref(path) && root.join(path).is_file())
        });
}

fn upsert_ref(root: &Path, row: &mut Value, key: &str, path: &str) {
    let Some(got) = digest(root, path) else {
        return;
    };
    let refs = row[key].as_array_mut().expect("artifact refs");
    if let Some(existing) = refs
        .iter_mut()
        .find(|item| item.get("path").and_then(Value::as_str) == Some(path))
    {
        existing["digest"] = json!(got);
    } else {
        refs.push(json!({"path": path, "digest": got}));
    }
}

fn legacy_agent_ref(path: &str) -> bool {
    path.starts_with("agents/") || path.starts_with("custom-agents/")
}

fn digest(root: &Path, path: &str) -> Option<String> {
    crate::digest::file(&root.join(path)).ok()
}

#[test]
fn round_receipt_reference_sync_skips_missing_unknown_and_unowned_rows() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let mut missing_role = json!({});
    sync_persona_refs(&root, &mut missing_role);
    assert!(missing_role.get("agent_manifest_path").is_none());

    let mut unknown_role = json!({"role":"unknown-reviewer"});
    sync_persona_refs(&root, &mut unknown_role);
    assert!(unknown_role.get("agent_manifest_path").is_none());

    let mut other_reviewer = json!({"role":"claim-falsifier"});
    bind_product_fitness(&root, &mut other_reviewer);
    assert!(
        other_reviewer
            .get("product_fitness_receipt_digest")
            .is_none()
    );
}

#[test]
fn round_receipt_sync_replaces_legacy_identity_with_current_manifest() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let mut row = json!({
        "role":"claim-falsifier",
        "persona_prompt_path":"agents/contract-claim-falsifier.md",
        "custom_agent_path":"custom-agents/harness-contract-claim-falsifier.toml",
        "proof_anchors_checked":[],
        "evidence_artifacts_checked":[]
    });
    sync_persona_refs(&root, &mut row);
    assert_eq!(
        row["agent_manifest_path"],
        json!(".codex/agents/claim-falsifier.toml")
    );
    assert!(row.get("persona_prompt_path").is_none());
    assert!(row.get("custom_agent_path").is_none());
    for key in ["proof_anchors_checked", "evidence_artifacts_checked"] {
        assert!(row[key].as_array().unwrap().iter().all(|item| {
            let path = item["path"].as_str().unwrap();
            !path.starts_with("agents/") && !path.starts_with("custom-agents/")
        }));
    }
}
