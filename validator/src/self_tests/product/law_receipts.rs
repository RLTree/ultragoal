use serde_json::json;
use std::path::Path;

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}

fn has(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

#[test]
fn execplan_memory_product_fitness_and_journey_branches_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("product-law-receipts");
    write_text(&root.join("live.txt"), "live");
    let execplan =
        json!({"schema":"wrong", "required_sections": {}, "stale": true, "prose_only": true});
    let execplan_failures =
        crate::audit::law::surface::receipt::workflow::restartable_execplan_value_failures(
            &root, &execplan,
        );
    assert!(has(&execplan_failures, "restartable_execplan_wrong_schema"));
    assert!(has(
        &execplan_failures,
        "restartable_execplan_missing_section:working_behavior"
    ));
    assert!(has(&execplan_failures, "restartable_execplan_stale"));
    assert!(has(&execplan_failures, "restartable_execplan_prose_only"));
    assert!(has(
        &execplan_failures,
        "restartable_execplan_claim_ceiling_not_enforced"
    ));

    let memory = json!({
        "schema":"wrong",
        "memory_context_only": false,
        "substitutions_rejected": ["memory_only"],
        "memory_artifacts": [{}],
        "live_same_surface_evidence": [{"path":"live.txt", "digest": crate::self_tests::boundaries::workspace_fixtures::sha('2')}],
        "claim_ceiling": "summary"
    });
    let memory_failures =
        crate::audit::law::surface::receipt::workflow::memory_context_value_failures(
            &root, &memory,
        );
    for expected in [
        "memory_context_wrong_schema",
        "memory_context_not_marked_context_only",
        "memory_context_substitute_not_rejected:summary_only",
        "memory_context_artifact_digest_mismatch",
        "memory_context_live_evidence_digest_mismatch",
        "memory_context_claim_ceiling_not_same_surface",
    ] {
        assert!(
            has(&memory_failures, expected),
            "{expected}: {memory_failures:?}"
        );
    }

    let mut evidence_failures = Vec::new();
    crate::audit::product::fitness::evidence::failures(
        &root,
        &json!({
            "bad": {"evidence": []},
            "empty": {"evidence": {}},
            "escape": {"evidence": {"path":"../escape", "digest": crate::self_tests::boundaries::workspace_fixtures::sha('3')}},
            "missing": {"evidence": {"path":"missing.txt", "digest": crate::self_tests::boundaries::workspace_fixtures::sha('4')}},
            "mismatch": {"evidence": {"path":"live.txt", "digest": crate::self_tests::boundaries::workspace_fixtures::sha('5')}}
        }),
        "",
        &mut evidence_failures,
    );
    for expected in [
        "product_fitness_evidence_untyped:/bad/evidence",
        "product_fitness_evidence_untyped:/empty/evidence",
        "product_fitness_evidence_path_invalid:/escape/evidence",
        "product_fitness_evidence_missing:/missing/evidence",
        "product_fitness_evidence_digest_mismatch:/mismatch/evidence",
    ] {
        assert!(
            has(&evidence_failures, expected),
            "{expected}: {evidence_failures:?}"
        );
    }

    let receipt = json!({
        "schema": "harness-ultragoal.product-fitness-receipt.v1",
        "claim": {"id": "CLAIM-001", "repeated_use_claimed": true},
        "target_revision": {"value": "sha256:missing"},
        "target_audience": {"name": "generic users"},
        "job_to_be_done": {"job": "job"},
        "context_of_use": {"context": "context"},
        "desired_user_outcome": {"outcome": "outcome"},
        "business_or_mission_outcome": {"outcome": "mission"},
        "critical_journey": {"id": "journey"},
        "proof_surface": {"kind": "quality_in_use"},
        "claim_ceiling": "withheld",
        "producer_actor_id": "same",
        "reviewer_actor_id": "same",
        "receipt_digest": crate::digest::ZERO
    });
    let receipt_failures = crate::audit::product::fitness::receipt::failures(&root, &receipt);
    assert!(has(&receipt_failures, "product_fitness_generic_user_claim"));
    assert!(has(
        &receipt_failures,
        "product_fitness_receipt_malformed:zero_digest"
    ));

    let short_journey = crate::audit::plugin::product::journey::failures(&root, &json!({}));
    assert_eq!(short_journey, vec!["plugin_product_journey_incomplete"]);
    let journey = json!({
        "journey": [0,1,2,3,4,5,6,7,8,9],
        "generated_at": "2026-06-24T00:00:00Z",
        "claim_ceiling": "wide",
        "evidence": [{"path":"../escape", "digest": crate::self_tests::boundaries::workspace_fixtures::sha('6')}],
        "error_path_evidence": {"path":"missing.txt", "digest": crate::self_tests::boundaries::workspace_fixtures::sha('7')}
    });
    let journey_failures = crate::audit::plugin::product::journey::failures(&root, &journey);
    for expected in [
        "plugin_product_journey_claim_ceiling_missing",
        "plugin_product_journey_placeholder_timestamp",
        "plugin_product_journey_evidence_incomplete",
        "plugin_product_journey_evidence_invalid",
    ] {
        assert!(
            has(&journey_failures, expected),
            "{expected}: {journey_failures:?}"
        );
    }
    assert!(has(
        &crate::audit::plugin::product::journey::failures(
            &root,
            &json!({"journey":[0,1,2,3,4,5,6,7,8,9],"claim_ceiling":"package_static_fixture_only","evidence":[]})
        ),
        "plugin_product_journey_error_path_missing"
    ));
    std::fs::remove_dir_all(root).expect("cleanup product law receipts");
}
