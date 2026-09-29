use serde_json::json;

fn root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(root.join("validation_artifacts/improvement-loop"))
        .expect("improvement loop dir");
    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    root
}

#[test]
fn improvement_loop_audit_rejects_missing_and_wrong_schema_receipts() {
    let root = root("audit-improvement-loop");
    let missing = super::package_failures(&root);
    assert!(
        missing
            .iter()
            .any(|failure| failure.starts_with("improvement_loop_receipt_missing_or_malformed")),
        "{missing:#?}"
    );

    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join(super::RECEIPT_REL),
        &json!({"schema":"wrong","status":"pass"}),
    )
    .expect("wrong schema");
    let wrong_schema = super::package_failures(&root);
    assert!(
        wrong_schema
            .iter()
            .any(|failure| failure == "improvement_loop_receipt_wrong_schema"),
        "{wrong_schema:#?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn copy_learning_surfaces(root: &std::path::Path) {
    let source_root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    std::fs::create_dir_all(root.join("docs/improvement-loop")).expect("projection dir");
    std::fs::copy(
        source_root.join(super::REGISTRY),
        root.join(super::REGISTRY),
    )
    .expect("registry");
    std::fs::copy(
        source_root.join(super::PROJECTION_REL),
        root.join(super::PROJECTION_REL),
    )
    .expect("projection");
}

#[test]
fn learning_adoption_accepts_one_adopted_and_one_held_record() {
    let root = root("learning-adoption-green");
    copy_learning_surfaces(&root);
    let failures = super::knowledge::failures(&root);
    assert!(failures.is_empty(), "{failures:#?}");
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn learning_projection_rejects_stale_or_hand_edited_bytes() {
    let root = root("learning-projection-tamper");
    copy_learning_surfaces(&root);
    let projection = root.join(super::PROJECTION_REL);
    let mut bytes = std::fs::read(&projection).expect("projection bytes");
    bytes.extend_from_slice(b"\nhand edit\n");
    std::fs::write(&projection, bytes).expect("tamper projection");
    let failures = super::knowledge::failures(&root);
    assert!(
        failures.iter().any(|failure| {
            failure == "improvement_loop_knowledge_projection_stale_or_hand_edited"
        }),
        "{failures:#?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn learning_adoption_rejects_universal_repair_budget_and_missing_eval_family() {
    let root = root("learning-adoption-budget");
    copy_learning_surfaces(&root);
    let registry_path = root.join(super::REGISTRY);
    let mut registry = crate::json_boundary::read_json(&registry_path).expect("registry JSON");
    registry["learning_adoption"]["records"][0]["repair_budget"]["universal_count"] = json!(true);
    registry["learning_adoption"]["records"][1]["evaluation"]
        .as_object_mut()
        .expect("evaluation")
        .remove("semantic_mutation");
    crate::self_tests::boundaries::workspace_fixtures::write_json(&registry_path, &registry)
        .expect("mutated registry");
    let failures = super::knowledge::failures(&root);
    assert!(
        failures
            .iter()
            .any(|failure| failure == "improvement_loop_learning_repair_budget_invalid:knowledge-projection-stale-mismatch"),
        "{failures:#?}"
    );
    assert!(
        failures.iter().any(|failure| {
            failure == "improvement_loop_learning_evaluation_missing:universal-repair-budget:semantic_mutation"
        }),
        "{failures:#?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn learning_adoption_rejects_duplicate_record_ids() {
    let root = root("learning-adoption-duplicate-id");
    copy_learning_surfaces(&root);
    let registry_path = root.join(super::REGISTRY);
    let mut registry = crate::json_boundary::read_json(&registry_path).expect("registry JSON");
    registry["learning_adoption"]["records"][1]["record_id"] =
        json!("knowledge-projection-stale-mismatch");
    crate::self_tests::boundaries::workspace_fixtures::write_json(&registry_path, &registry)
        .expect("mutated registry");
    let failures = super::knowledge::failures(&root);
    assert!(
        failures.iter().any(|failure| {
            failure
                == "improvement_loop_learning_record_id_duplicate:knowledge-projection-stale-mismatch"
        }),
        "{failures:#?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
