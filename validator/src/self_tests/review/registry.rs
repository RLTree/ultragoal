use serde_json::{Value, json};
use std::path::Path;

#[cfg(unix)]
mod reader;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn errors(out: &[crate::review::round::ReviewFailure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

fn copy_bound_inputs(root: &Path) {
    let live = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    for relative in std::iter::once("schemas/codex-registry-exposure.schema.json").chain(
        crate::review::round::config::REVIEW_ROLES
            .iter()
            .map(|spec| spec.agent_manifest_path),
    ) {
        let target = root.join(relative);
        std::fs::create_dir_all(target.parent().unwrap()).expect("bound input parent");
        std::fs::copy(live.join(relative), target).expect("bound input");
    }
}

fn fail_exposure() -> Value {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    crate::json_boundary::read_json(
        &root.join("fixtures/review-round/anchors/live-registry-exposure.json"),
    )
    .expect("fail-only registry fixture")
}

fn direct_errors(root: &Path, exposure: &Value) -> Vec<crate::review::round::ReviewFailure> {
    let path = root.join("validation_artifacts/exposure.json");
    write_json(&path, exposure);
    let digest = crate::digest::file(&path).expect("exposure digest");
    let mut out = Vec::new();
    crate::review::round::registry::exposure_errors(
        root,
        &json!({
            "generated_at":"2026-06-18T00:00:00Z",
            "round_id":"fixture-canonical-falsification-round",
            "live_registry_exposure":{"path":"validation_artifacts/exposure.json","digest":digest}
        }),
        &mut out,
    );
    out
}

#[test]
fn schema_valid_unavailable_registry_blocks_and_positive_substitutes_cannot_pass() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("registry-fail-only");
    copy_bound_inputs(&root);
    let fail = direct_errors(&root, &fail_exposure());
    assert_eq!(
        fail.first().map(|failure| failure.error.as_str()),
        Some("review_round_live_registry_unavailable")
    );

    let mut claimed_pass = fail_exposure();
    claimed_pass["status"] = json!("pass");
    claimed_pass["issuer"] = json!({"tool":"multi_agent_v1","authority":"tool_registry"});
    claimed_pass["tool_call"]["name"] = json!("multi_agent_v1.tool_registry");
    claimed_pass["capture_method"] = json!("live_tool_registry_query");
    claimed_pass["source"] = json!("multi_agent_v1.tool_registry");
    claimed_pass["claim_ceiling"] = json!("live_registry_reviewer_exposure_proven");
    let pass_errors = direct_errors(&root, &claimed_pass);
    assert!(
        errors(&pass_errors).contains(&"review_round_live_registry_artifact_malformed"),
        "{:?}",
        errors(&pass_errors)
    );
    assert!(!pass_errors.is_empty());

    let mut exposed = claimed_pass;
    for row in exposed["agent_types"].as_array_mut().unwrap() {
        row["exposed"] = json!(true);
    }
    let exposed_errors = direct_errors(&root, &exposed);
    assert_eq!(
        exposed_errors.first().map(|failure| failure.error.as_str()),
        Some("review_round_live_registry_artifact_malformed")
    );
    std::fs::remove_dir_all(root).expect("cleanup fail-only registry");
}

#[test]
fn live_registry_exposure_uses_typed_current_run_freshness_window() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("registry-freshness");
    copy_bound_inputs(&root);
    for (captured_at, stale) in [
        ("2026-06-18T00:00:00Z", false),
        ("2026-06-17T23:54:59Z", true),
        ("2026-06-18T00:00:01Z", true),
        ("not-a-time-0000000000", true),
    ] {
        let mut exposure = fail_exposure();
        exposure["captured_at"] = json!(captured_at);
        let out = direct_errors(&root, &exposure);
        assert_eq!(
            errors(&out).contains(&"review_round_live_registry_stale"),
            stale,
            "{captured_at}: {:?}",
            errors(&out)
        );
        assert!(errors(&out).contains(&"review_round_live_registry_unavailable"));
    }
    std::fs::remove_dir_all(root).expect("cleanup registry freshness");
}

#[test]
fn canonical_registry_negative_anchors_preserve_their_causal_first_failure() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let base = crate::json_boundary::read_json(
        &root.join("fixtures/review-round/valid/review-round-receipt.json"),
    )
    .expect("review receipt");
    for (name, expected) in [
        (
            "live-registry-exposure-stale.json",
            "review_round_live_registry_stale",
        ),
        (
            "live-registry-exposure-stale-captured-at.json",
            "review_round_live_registry_stale",
        ),
        (
            "live-registry-exposure-disk-synced-active-stale.json",
            "review_round_live_registry_unavailable",
        ),
    ] {
        let rel = format!("fixtures/review-round/anchors/{name}");
        let mut receipt = base.clone();
        receipt["live_registry_exposure"] = json!({
            "path":rel.clone(),
            "digest":crate::digest::file(&root.join(&rel)).expect("anchor digest")
        });
        let mut out = Vec::new();
        crate::review::round::registry::exposure_errors(&root, &receipt, &mut out);
        assert_eq!(
            out.first().map(|failure| failure.error.as_str()),
            Some(expected),
            "{name}: {:?}",
            errors(&out)
        );
    }
}

#[test]
fn registry_role_mismatch_uses_fixed_non_echo_detail() {
    let mut out = Vec::new();
    crate::review::round::registry::row_agent_role_error(
        &json!({"role":"SECRET_CANARY"}),
        "claim-falsifier",
        &mut out,
    );
    assert_eq!(out[0].error, "review_round_live_registry_agent_mismatch");
    assert_eq!(out[0].detail, "reviewer-role");
}
