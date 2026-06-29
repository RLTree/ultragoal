use crate::audit::contract::{CHECK_IDS, VERSION};
use crate::digest;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::Path;

mod execution;

#[cfg(test)]
pub(crate) fn bind(
    root: &Path,
    value: &Value,
    validator_digests: &BTreeMap<String, String>,
) -> Value {
    let target_digest = crate::package::inventory::package_digest(root).unwrap_or_default();
    bind_with_candidate(root, value, validator_digests, &target_digest)
}

#[cfg(test)]
pub(crate) fn bind_with_candidate(
    root: &Path,
    value: &Value,
    validator_digests: &BTreeMap<String, String>,
    target_digest: &str,
) -> Value {
    let mut digest_cache = BTreeMap::new();
    let red_fixture_ids = red_ids(root);
    let red_fixture_rows = red_fixtures_with_ids(root, &red_fixture_ids, &mut digest_cache);
    let input_digest_rows = input_digests_with_cache(root, &mut digest_cache);
    bind_with_candidate_and_cache(
        root,
        value,
        validator_digests,
        target_digest,
        &mut digest_cache,
        &red_fixture_ids,
        &red_fixture_rows,
        &input_digest_rows,
    )
}

pub(crate) fn bind_with_candidate_and_cache(
    root: &Path,
    value: &Value,
    validator_digests: &BTreeMap<String, String>,
    target_digest: &str,
    digest_cache: &mut BTreeMap<String, String>,
    red_fixture_ids: &[String],
    red_fixture_rows: &Value,
    input_digest_rows: &[Value],
) -> Value {
    if value.get("schema").and_then(Value::as_str) != Some("harness-ultragoal.fixture-bundle.v1") {
        return value.clone();
    }
    let mut bound = value.clone();
    let run_id = run_id(&bound);
    let validator_artifacts = validator_artifacts(validator_digests);
    let root_identity = crate::audit::receipt::root_identity(root);
    let validator_execution = execution::value(
        root,
        &run_id,
        &validator_artifacts,
        input_digest_rows,
        digest_cache,
    );
    let red_fixture_catalog_digest = crate::red::fixture::runtime::artifact::digest_or_zero_cached(
        root,
        "templates/RED_FIXTURES.json",
        digest_cache,
    );
    bound["validator_receipt"] = json!({
        "schema": "harness-ultragoal.validator-receipt.v1",
        "validator": "ultragoal-audit",
        "version": VERSION,
        "run_id": run_id,
        "target": root_identity,
        "status": "pass",
        "commit": target_digest,
        "target_revision": {"kind": "package_digest", "value": target_digest},
        "claim_ceiling": "source_audit_pass_source_local_only",
        "supported_claim_classes": ["source_local_audit_checks", "red_fixture_report"],
        "blocked_claim_classes": [
            "completion",
            "package_readiness",
            "review_readiness",
            "release_readiness",
            "final_packet_correctness",
            "update_goal_eligibility",
            "app_registry_or_reviewer_exposure"
        ],
        "blocked_claim_diagnostics": [{
            "surface": "final_packet_proof",
            "path": "validation_artifacts/review/final-packet-proof.json",
            "digest": digest::ZERO,
            "status": "clear",
            "observed_failures": [],
            "blocked_claim_classes": [
                "completion",
                "package_readiness",
                "review_readiness",
                "release_readiness",
                "final_packet_correctness",
                "update_goal_eligibility",
                "app_registry_or_reviewer_exposure"
            ],
            "claim_impact": "source_audit_pass_does_not_support_final_packet_registry_readiness_release_completion_or_update_goal"
        }],
        "root": root_identity,
        "validator_execution": validator_execution,
        "input_digests": input_digest_rows,
        "required_execplan_refs": required_execplan_refs(),
        "required_check_ids": required_check_ids(),
        "check_set_digest": check_set_digest(),
        "checks": checks(),
        "required_red_fixture_ids": red_fixture_ids,
        "red_fixture_catalog_digest": red_fixture_catalog_digest,
        "red_fixtures": red_fixture_rows,
        "generated_artifacts": crate::red::fixture::runtime::artifact::generated_artifacts_cached(root, &run_id, digest_cache),
        "generated_at": crate::audit::clock::now_iso()
    });
    bound
}

fn run_id(value: &Value) -> String {
    value
        .pointer("/ready_for_merge/validator_run_id")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .pointer("/validator_receipt/run_id")
                .and_then(Value::as_str)
        })
        .unwrap_or("fixture-runtime")
        .to_string()
}

fn validator_artifacts(validator_digests: &BTreeMap<String, String>) -> Vec<Value> {
    validator_digests
        .iter()
        .map(|(path, digest)| json!({"path": path, "digest": digest}))
        .collect()
}

pub(crate) fn input_digests_with_cache(
    root: &Path,
    digest_cache: &mut BTreeMap<String, String>,
) -> Vec<Value> {
    let mut rows = Vec::new();
    for rel in [
        "plugin-manifest-draft.json",
        "schemas/schema-catalog.json",
        "templates/RED_FIXTURES.json",
        "fixtures/valid/minimal-goal-run.json",
    ] {
        let Ok(path) = crate::package::inventory::resolve(root, rel) else {
            return vec![crate::red::fixture::runtime::artifact::ref_value_cached(
                root,
                "fixtures/valid/minimal-goal-run.json",
                digest_cache,
            )];
        };
        let digest = if let Some(cached) = digest_cache.get(rel) {
            cached.clone()
        } else {
            let digest = crate::audit::artifacts::artifact_digest_or_zero(&path)
                .unwrap_or_else(|_| digest::ZERO.to_string());
            digest_cache.insert(rel.to_string(), digest.clone());
            digest
        };
        rows.push(json!({"path": rel, "digest": digest}));
    }
    rows
}

fn required_execplan_refs() -> Vec<&'static str> {
    vec![
        "harness-ultragoal-plans-and-orchestrator-automation-hardening.md",
        "mandatory-coverage-authority-and-enforcement.md",
        "mandatory-coverage-scope-authority-and-anti-theater.md",
        "mandatory-plugin-product-cohesion-and-fit-repo-authority.md",
        "mandatory-product-fitness-quality-in-use-enforcement.md",
    ]
}

fn required_check_ids() -> Vec<&'static str> {
    CHECK_IDS.to_vec()
}

fn check_set_digest() -> String {
    digest::bytes(
        serde_json::to_string(&required_check_ids())
            .unwrap_or_default()
            .as_bytes(),
    )
}

fn checks() -> Value {
    CHECK_IDS
        .iter()
        .map(|id| {
            (
                (*id).to_string(),
                json!({"status": "pass", "details": "runtime-bound fixture receipt"}),
            )
        })
        .collect::<Map<String, Value>>()
        .into()
}

#[cfg(test)]
fn red_ids(root: &Path) -> Vec<String> {
    crate::audit::artifacts::safe_red_ids(root)
}

pub(crate) fn red_fixtures_with_ids(
    root: &Path,
    red_fixture_ids: &[String],
    digest_cache: &mut BTreeMap<String, String>,
) -> Value {
    let digest = crate::red::fixture::runtime::artifact::digest_or_zero_cached(
        root,
        "templates/RED_FIXTURES.json",
        digest_cache,
    );
    red_fixture_ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                json!({
                    "packet_path": "templates/RED_FIXTURES.json",
                    "packet_digest": digest,
                    "expected_error": "runtime_bound_fixture_receipt",
                    "expected_failing_check": "validator-execution-provenance",
                    "observed_error": "runtime_bound_fixture_receipt",
                    "observed_failing_check": "validator-execution-provenance",
                    "validator_exit": 1,
                    "status": "pass"
                }),
            )
        })
        .collect::<Map<String, Value>>()
        .into()
}
