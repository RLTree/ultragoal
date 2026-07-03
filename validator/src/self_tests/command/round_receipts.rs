use serde_json::{Value, json};
use std::path::Path;

pub(crate) fn sync_persona_refs(root: &Path, row: &mut Value) {
    let Some(persona) = row.get("persona").and_then(Value::as_str) else {
        return;
    };
    let Some(spec) = crate::review::round::config::persona_spec(persona) else {
        return;
    };
    for (path_key, digest_key, path) in [
        (
            "persona_prompt_path",
            "persona_prompt_digest",
            spec.prompt_path,
        ),
        ("custom_agent_path", "custom_agent_digest", spec.custom_path),
    ] {
        row[path_key] = json!(path);
        row[digest_key] = json!(digest(root, path));
    }
    for key in ["proof_anchors_checked", "evidence_artifacts_checked"] {
        retain_existing_refs(root, row, key);
        for path in [spec.prompt_path, spec.custom_path, spec.focus_path] {
            upsert_ref(root, row, key, path);
        }
    }
}

pub(crate) fn bind_product_fitness(root: &Path, row: &mut Value) {
    if row.get("persona").and_then(Value::as_str) != Some("product_simplicity_falsifier") {
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
                .is_some_and(|path| root.join(path).is_file())
        });
}

fn upsert_ref(root: &Path, row: &mut Value, key: &str, path: &str) {
    let got = digest(root, path);
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

fn digest(root: &Path, path: &str) -> String {
    crate::digest::file(&root.join(path)).expect(path)
}

#[test]
fn round_receipt_reference_sync_skips_missing_unknown_and_unowned_rows() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let mut missing_persona = json!({});
    sync_persona_refs(&root, &mut missing_persona);
    assert!(missing_persona.get("persona_prompt_path").is_none());

    let mut unknown_persona = json!({"persona":"unknown-reviewer"});
    sync_persona_refs(&root, &mut unknown_persona);
    assert!(unknown_persona.get("persona_prompt_path").is_none());

    let mut other_reviewer = json!({"persona":"contract_claim_falsifier"});
    bind_product_fitness(&root, &mut other_reviewer);
    assert!(
        other_reviewer
            .get("product_fitness_receipt_digest")
            .is_none()
    );
}
