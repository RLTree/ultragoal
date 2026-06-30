use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}

fn contains(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

#[test]
fn template_integrity_reads_files_and_rejects_stateful_placeholders() {
    let root = crate::self_tests::boundaries::support::temp_root("template-integrity");
    let missing = crate::audit::template_integrity::package_failures(&root);
    assert!(contains(&missing, "template_missing:templates/PLANS.md"));

    write_text(
        &root.join("templates/PLANS.md"),
        "stable ExecPlan law\nnot the project plan ledger\ndocs/exec-plans/active/*\n\
         ## Non-Negotiable Requirements\n## Required Sections\n## Lane Extension\n\
         ## Orchestrator Responsibilities\n## Lane Ready Message\nworker thread:\n",
    );
    write_text(
        &root.join("templates/.codex/automations/ultragoal-orchestrator/automation.toml"),
        "target_thread_id='__THREAD__'\ngoal_id='g'\nrepo_root='.'\n\
         DONT_NOTIFY='x'\nSTEER='x'\nESCALATE='x'\nautomation_tick_receipt_path='tick.json'\n",
    );
    write_text(
        &root.join("templates/.codex/automations/ultragoal-orchestrator/prompt.md"),
        "first-wave lane\napp automation tools\napp thread tools\nvalidator commands\n\
         harness-ultragoal:ultragoal\nharness-ultragoal:orchestrator-reconciler\n\
         harness-ultragoal:execplan-lane\nharness-ultragoal:proof-gate\n\
         harness-ultragoal:standards-gardener\nharness-ultragoal:harness-engineering\n\
         harness-ultragoal:agent-first-repo-init\nharness-ultragoal:agent-first-repo-retrofit\n\
         goal/contract cursor\nlane/session cursor\nrepo/artifact cursor\n\
         automation/thread cursor\nchanged artifacts or receipts\n\
         Mutation requires explicit contract authority\nkeep working __PROMPT__\n",
    );
    let failures = crate::audit::template_integrity::package_failures(&root);
    assert!(contains(&failures, "plans_contains_project_state"));
    let active = crate::audit::template_integrity::value_failures(&json!({
        "plans_md":"",
        "automation_toml":"target_thread_id __THREAD__",
        "automation_prompt":"keep working __PROMPT__",
        "activation_candidate":true
    }));
    assert!(contains(
        &active,
        "automation_unresolved_placeholder_active"
    ));
    assert!(contains(&active, "automation_generic_reminder_prompt"));
    std::fs::remove_dir_all(root).expect("cleanup template integrity");
}

#[test]
fn package_run_records_nonpassing_red_fixture_results() {
    let root = crate::self_tests::boundaries::support::temp_root("package-run-red-fail");
    for dir in [
        "fixtures/valid",
        "schemas",
        "templates",
        "validation_artifacts/ultragoal-audit",
        "validator/src",
        "examples/generated",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"name":"red-fail","version":"0.0.0","resources":[]}),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[]}),
    );
    write_json(
        &root.join("fixtures/valid/minimal-goal-run.json"),
        &json!({}),
    );
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{
            "id":"red-mismatch",
            "packet_path":"fixtures/red/missing.json",
            "expected_failure":{"check_id":"schema-valid","error":"different_error"}
        }]),
    );
    let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let red_report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    let code = crate::audit::package::run::run(
        crate::audit::AuditOptions {
            root: root.clone(),
            receipt: receipt.clone(),
            red_report: Some(red_report.clone()),
            target_repo: None,
            mode: "source".to_string(),
            require_observability: false,
            require_product_cohesion: false,
            jobs: None,
            command_text: "ultragoal source audit --unit".to_string(),
        },
        red_report.clone(),
    )
    .expect("package run writes fail receipt");
    assert_eq!(code, 1);
    let receipt_json = crate::json_boundary::read_json(&receipt).expect("receipt");
    assert!(
        receipt_json["checks"]["red-fixture-coverage"]["details"]
            .as_str()
            .unwrap_or("")
            .contains("one or more red fixtures did not fail as expected")
    );
    let red = crate::json_boundary::read_json(&red_report).expect("red report");
    assert_eq!(red["red_fixtures"]["red-mismatch"]["status"], "fail");
    std::fs::remove_dir_all(root).expect("cleanup package run red fail");
}

#[test]
fn semantic_claim_ids_and_text_surface_fallbacks_fail_closed() {
    let root = crate::self_tests::boundaries::support::temp_root("semantic-text-extra");
    std::fs::create_dir_all(&root).expect("root");
    let mut bundle = crate::json_boundary::read_json(
        &crate::self_tests::boundaries::support::repo_root()
            .join("fixtures/valid/minimal-goal-run.json"),
    )
    .expect("minimal fixture");
    let mut duplicate = bundle["completion_manifest"]["claims"][0].clone();
    duplicate["title"] = json!("Codex picked plugin store without promotion receipt");
    duplicate["description"] = json!("The plugin released to the team library without proof");
    duplicate["status"] = json!("pass");
    duplicate["claim_ceiling_effect"] = json!("included");
    duplicate["evidence"] = json!([]);
    bundle["completion_manifest"]["claims"]
        .as_array_mut()
        .expect("claims")
        .push(duplicate);
    let errors = crate::claim_semantics::semantic_failures(&bundle, &root, &BTreeMap::new())
        .into_iter()
        .map(|failure| failure.error)
        .collect::<Vec<_>>();
    assert!(errors.contains(&"duplicate_completion_claim_id".to_string()));
    assert!(errors.contains(&"claim_text_requires_unproven_surface".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup semantic text extra");
}

#[test]
fn standards_trace_and_audit_receipt_edges_cover_current_artifacts() {
    let root = crate::self_tests::boundaries::support::temp_root("trace-audit-edges");
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[{"id":"law1"}]}),
    );
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{"id":"red1"}]),
    );
    write_json(&root.join("fixtures/valid/v.json"), &json!({}));
    write_text(&root.join("source.md"), "source");
    let source_digest = crate::digest::file(&root.join("source.md")).expect("source digest");
    let matrix = json!({"obligations":[{"id":"law1"}]});
    write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[{
            "obligation_id":"law1",
            "source_artifact":{"path":"source.md","digest":source_digest},
            "law_id":"law1",
            "standards_row_id":"law1",
            "validator_check_id":"schema-valid",
            "red_fixture_id":"red1",
            "valid_fixture_id":"fixtures/valid/v.json",
            "receipt_requirement":"receipt",
            "claim_ceiling_impact":"blocks"
        }]}),
    );
    assert!(crate::audit::foundational_law_trace::failures(&root, &matrix).is_empty());

    write_json(
        &root.join("docs/law.json"),
        &json!({"generated_at":"2026-06-25T00:00:00Z"}),
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let artifact_digest = crate::digest::file(&root.join("docs/law.json")).expect("law digest");
    let candidate_digest = crate::package::inventory::package_digest(&root).expect("digest");
    write_json(
        &root.join("receipts/gardener.json"),
        &json!({
            "schema":"harness-ultragoal.standards-gardening-receipt.v1",
            "status":"pass",
            "generated_at":"2026-06-25T00:10:00Z",
            "trigger_signal":{"signal_id":"sig","severity":"moderate","signal_kind":"standards_entropy","summary":"summary","source":"test"},
            "decision":{"accepted":true,"action":"validator_check","rationale":"because"},
            "changed_artifacts":[{"path":"docs/law.json","digest":artifact_digest}],
            "candidate_digest":candidate_digest,
            "safeguards":{"deterministic_first":true,"no_hook_by_default":true},
            "claim_ceiling":"package_static_fixture_only"
        }),
    );
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[{
            "id":"standards-gardener-promotion",
            "standards_gardening_receipt_path":"receipts/gardener.json"
        }]}),
    );
    assert!(crate::audit::standards_gardening::failures(&root, &store).is_empty());
    assert_eq!(
        crate::audit::agent::standards::tsv::checks::audit_row_failures(
            &root,
            &json!({"standard_id":"law1","audit_status":"pass","evidence_digest":crate::digest::ZERO})
        ),
        vec!["agent_standards_audit_evidence_invalid:law1"]
    );
    std::fs::remove_dir_all(root).expect("cleanup trace audit edges");
}
