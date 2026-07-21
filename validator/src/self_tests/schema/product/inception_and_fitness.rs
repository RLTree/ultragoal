use serde_json::{Value, json};

#[test]
fn historical_and_evidence_led_product_briefs_have_distinct_valid_shapes() {
    let store = store();
    let v1 = json!({
        "schema": "harness-ultragoal.product-success-brief.v1",
        "product_success_contract_id": "PSC-1",
        "claim_ids": ["CL-SOURCE"],
        "target_problem": "Deliver one useful repository change",
        "audience": "Repository operator",
        "job_to_be_done": "Complete the change safely",
        "context_of_use": "A real repository",
        "desired_outcome": "A verified useful outcome",
        "first_value_event": "The first useful change passes verification",
        "evidence_ladder": "intent",
        "claim_ceiling": "intent only"
    });
    assert_valid(&store, "product-success-brief.schema.json", &v1);

    let v2 = json!({
        "schema": "harness-ultragoal.product-success-brief.v2",
        "product_success_contract_id": "PSC-1",
        "product_success_contract_digest": digest('1'),
        "claim_ids": ["CL-SOURCE"],
        "target_problem": "Deliver one useful repository change",
        "audience": "Repository operator",
        "job_to_be_done": "Complete the change safely",
        "context_of_use": "A real repository",
        "desired_outcome": "A verified useful outcome",
        "first_value_event": "The first useful change passes verification",
        "operator": {"kind": "agent", "actor_reference": "operator-1"},
        "real_work": {
            "repository_identity": digest('2'),
            "starting_candidate": digest('3'),
            "dirty_state_expectation": "dirty",
            "task_id": "task-1",
            "task": "Apply one bounded repair",
            "expected_useful_outcome": "The repair works and unrelated state remains"
        },
        "public_entry_surface": {
            "surface_id": "PS-ENTRY",
            "route": "harness-ultragoal",
            "forbidden_bypasses": ["direct internal command"]
        },
        "protected_invariants": [{
            "id": "preserve-unrelated-state",
            "claim_ids": ["CL-SOURCE"],
            "surfaces": ["source"],
            "required_condition": "Unrelated bytes remain unchanged",
            "disposition": "fail_closed"
        }],
        "first_truth_loop": {
            "loop_id": "loop-1",
            "positive_path": [{
                "transition_id": "inspect",
                "action_id": "inspect-product-inception",
                "command_id": "inspect-inception",
                "effect": "read",
                "order": 1,
                "dependency_ids": [],
                "capability_ids": ["read-repository"],
                "claim_ids": ["CL-SOURCE"],
                "product_surfaces": ["source"],
                "expected_observation": "Current state is inspected without writes",
                "evidence_class": "source"
            }],
            "first_value_transition": "inspect",
            "failure_control": {
                "transition_id": "inspect",
                "failure": "The repository changes during inspection",
                "diagnosis": "Revalidate the candidate",
                "recovery": "Fail closed and retry from current state",
                "preservation": "No new writes are introduced"
            },
            "preservation_expectation": "Unrelated state remains unchanged",
            "repeat_use_expectation": "A later inspection is safely repeatable"
        },
        "depth_triggers": [{
            "trigger_id": "observed-failure",
            "action_id": "inspect-product-inception",
            "kind": "observed_failure",
            "risk_or_claim": "CL-SOURCE",
            "activation_finding_codes": ["candidate-changed"],
            "smallest_investment": "Revalidate once",
            "fitness_function": "Inspection remains zero-write",
            "invalidation_condition": "Candidate identity changes"
        }],
        "evidence_class": "source",
        "evidence_ladder": "source",
        "claim_ceiling": "source_only"
    });
    assert_valid(&store, "product-success-brief.schema.json", &v2);
}

#[test]
fn product_fitness_v2_requires_separate_truth_surfaces_and_real_use_fields() {
    let store = store();
    let mut receipt: Value = serde_json::from_slice(include_bytes!(
        "../../../../../validation_artifacts/harness/product-fitness-receipt.json"
    ))
    .expect("fitness receipt");
    receipt["schema"] = json!("harness-ultragoal.product-fitness-receipt.v2");
    receipt["operator_kind"] = json!("agent");
    receipt["evidence_class"] = json!("agent_use");
    let withheld = json!({"status": "withheld"});
    receipt["surface_identities"] = json!({
        "source": withheld,
        "package": {"status": "withheld"},
        "marketplace": {"status": "withheld"},
        "install": {"status": "withheld"},
        "cache": {"status": "withheld"},
        "app_registry": {"status": "withheld"},
        "discovery": {"status": "withheld"},
        "runtime": {"status": "withheld"},
        "journey": {"status": "withheld"}
    });
    receipt["public_entry_observation"] = json!({
        "surface_id": "PS-ENTRY", "route": "harness-ultragoal",
        "bypass_attempted": true, "bypass_rejected": true
    });
    receipt["real_work_observation"] = json!({
        "repository_identity": digest('4'), "task_id": "task-1",
        "task": "Apply one repair", "useful_outcome": "Repair verified"
    });
    receipt["manual_journey_row"] = json!({
        "time_to_verified_value_ms": 1, "human_interventions": 0,
        "review_rounds": 1, "failure": "", "diagnosis": "none",
        "recovery_outcome": "not needed", "repeat_use_outcome": "withheld",
        "retained_artifact_bytes": 0, "retained_cache_bytes": 0,
        "false_passes": 0, "false_rejections": 0
    });
    assert_valid(&store, "product-fitness-receipt.schema.json", &receipt);
}

fn store() -> crate::schema_catalog::SchemaStore {
    crate::schema_catalog::load(&crate::self_tests::boundaries::workspace_fixtures::repo_root())
}

fn assert_valid(store: &crate::schema_catalog::SchemaStore, schema: &str, value: &Value) {
    let errors = crate::schema_catalog::schema_errors(store, schema, value);
    assert!(errors.is_empty(), "{errors:?}");
}

fn digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}
