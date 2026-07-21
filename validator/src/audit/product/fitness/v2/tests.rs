use serde_json::{Value, json};

fn binding(path: &str, candidate: &str) -> Value {
    json!({
        "path": path,
        "digest": format!("sha256:{}", "a".repeat(64)),
        "candidate_id": candidate,
        "same_surface": true,
        "current_session": true
    })
}

fn observed(name: &str, candidate: &str) -> Value {
    json!({
        "status": "observed",
        "identity": format!("{name}-identity"),
        "evidence": binding(&format!("{name}.json"), candidate)
    })
}

fn receipt() -> Value {
    let candidate = format!("sha256:{}", "b".repeat(64));
    let repository = format!("sha256:{}", "c".repeat(64));
    json!({
        "target_revision": {"value": candidate},
        "claim": {"repeated_use_claimed": false},
        "operator_kind": "agent",
        "evidence_class": "installed",
        "surface_identities": {
            "source": observed("source", &candidate),
            "package": observed("package", &candidate),
            "marketplace": observed("marketplace", &candidate),
            "install": {"status": "withheld"},
            "cache": {"status": "withheld"},
            "app_registry": {"status": "withheld"},
            "discovery": {"status": "withheld"},
            "runtime": {"status": "withheld"},
            "journey": {"status": "withheld"}
        },
        "public_entry_observation": {
            "surface_id": "PS-ENTRY",
            "route": "harness-ultragoal",
            "bypass_attempted": true,
            "bypass_rejected": true,
            "evidence": binding("entry.json", &candidate)
        },
        "real_work_observation": {
            "repository_identity": repository,
            "repository_evidence": {
                "path": "repository.json",
                "digest": repository,
                "candidate_id": candidate,
                "same_surface": true,
                "current_session": true
            },
            "task_id": "task-1",
            "task": "perform one real repository task",
            "useful_outcome": "the task produces a verified outcome",
            "evidence": binding("work.json", &candidate)
        },
        "manual_journey_row": {
            "time_to_verified_value_ms": 1,
            "human_interventions": 0,
            "review_rounds": 1,
            "failure": "controlled failure",
            "diagnosis": "typed diagnosis",
            "recovery_outcome": "recovered",
            "repeat_use_outcome": "withheld",
            "retained_artifact_bytes": 0,
            "retained_cache_bytes": 0,
            "false_passes": 0,
            "false_rejections": 0
        },
        "claimed_surface": "marketplace",
        "surface_claim_ceilings": {
            "source": "live_same_surface_proven",
            "package": "live_same_surface_proven",
            "marketplace": "live_same_surface_proven",
            "install": "withheld_or_blocked",
            "cache": "withheld_or_blocked",
            "app_registry": "withheld_or_blocked",
            "discovery": "withheld_or_blocked",
            "runtime": "withheld_or_blocked",
            "journey": "withheld_or_blocked"
        },
        "claim_ceiling": "live_same_surface_proven",
        "substitution_rejections": [{
            "rejected_substitute": "legacy_dogfood_receipt",
            "reason": "cleanup evidence is not real-use evidence"
        }]
    })
}

#[test]
fn installed_marketplace_claim_does_not_require_human_journey() {
    assert!(super::failures(&receipt()).is_empty());
}

#[test]
fn runtime_claim_requires_its_live_predecessors() {
    let mut receipt = receipt();
    receipt["evidence_class"] = json!("runtime");
    receipt["claimed_surface"] = json!("runtime");
    receipt["surface_identities"]["runtime"] = observed(
        "runtime",
        receipt["target_revision"]["value"].as_str().unwrap(),
    );
    receipt["surface_claim_ceilings"]["runtime"] = json!("live_same_surface_proven");
    let failures = super::failures(&receipt);
    assert!(failures.iter().any(|failure| {
        failure == "product_fitness_claimed_surface_predecessor_missing:runtime:install"
    }));
    assert!(failures.iter().any(|failure| {
        failure == "product_fitness_claimed_surface_predecessor_missing:runtime:discovery"
    }));
}

#[test]
fn v2_requires_one_explicit_legacy_dogfood_rejection() {
    let mut receipt = receipt();
    receipt["substitution_rejections"] = json!([]);
    assert!(
        super::failures(&receipt)
            .iter()
            .any(|failure| { failure == "product_fitness_legacy_dogfood_substitution_missing" })
    );
}
