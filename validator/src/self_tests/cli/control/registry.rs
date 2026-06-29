use crate::cli::control::plane::types::ControlOperation;
use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn copy_schema_catalog(root: &Path) {
    let repo = crate::self_tests::boundaries::support::repo_root();
    let dst = root.join("schemas");
    std::fs::create_dir_all(&dst).expect("schema dst");
    for entry in std::fs::read_dir(repo.join("schemas")).expect("schemas") {
        let entry = entry.expect("schema entry");
        if entry.path().is_file() {
            std::fs::copy(entry.path(), dst.join(entry.file_name())).expect("copy schema");
        }
    }
}

#[test]
fn registry_probe_preserves_existing_live_same_surface_pass() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-registry-preserve-pass");
    copy_schema_catalog(&root);
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let raw_rel = "validation_artifacts/ultragoal-audit/live-registry-raw.json";
    write_json(&root.join(raw_rel), &raw_observation(&current));
    let raw_digest = crate::digest::file(&root.join(raw_rel)).expect("raw digest");
    let active_rel = "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";
    write_json(
        &root.join(active_rel),
        &live_registry_receipt(&current, raw_rel, &raw_digest),
    );
    let before = crate::digest::file(&root.join(active_rel)).expect("before digest");

    crate::cli::control::plane::registry::mint_fail_closed_if_needed(
        &root,
        ControlOperation::RegistryProbe,
        &current,
    )
    .expect("registry mint");

    let after = crate::digest::file(&root.join(active_rel)).expect("after digest");
    assert_eq!(after, before);
    assert!(
        !root
            .join("validation_artifacts/ultragoal-audit/active-registry-observation-current.json")
            .exists()
    );
    std::fs::remove_dir_all(root).expect("cleanup cli registry preserve");
}

#[test]
fn registry_fail_closed_boundary_normalizes_runtime_values() {
    let boundary = crate::cli::control::plane::registry::boundary_from_values(
        Some("acct-1"),
        Some("workspace.alpha"),
        Some("thread:019f"),
    );
    assert_eq!(boundary.account_id, "acct-1");
    assert_eq!(boundary.workspace_id, "workspace.alpha");
    assert_eq!(boundary.session_id, "thread:019f");

    let unsafe_boundary = crate::cli::control::plane::registry::boundary_from_values(
        Some("acct one"),
        Some(""),
        None,
    );
    assert_eq!(unsafe_boundary.account_id, "unavailable");
    assert_eq!(unsafe_boundary.workspace_id, "unavailable");
    assert_eq!(unsafe_boundary.session_id, "unavailable");
}

#[test]
fn registry_probe_fail_closed_receipt_uses_runtime_session_and_candidate_ids() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-registry-runtime-boundary");
    copy_schema_catalog(&root);
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    crate::cli::control::plane::registry::mint_fail_closed_if_needed(
        &root,
        ControlOperation::RegistryProbe,
        &current,
    )
    .expect("registry mint");

    let active_rel = "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";
    let active = crate::json_boundary::read_json(&root.join(active_rel)).expect("active receipt");
    let env_thread = std::env::var("CODEX_THREAD_ID").ok();
    let expected = crate::cli::control::plane::registry::boundary_from_values(
        None,
        None,
        env_thread.as_deref(),
    );
    assert_eq!(active["boundary"]["session_id"], expected.session_id);
    assert_eq!(active["session_id"], expected.session_id);
    assert_ne!(
        active["round_id"],
        json!("source-compliance-hardening-2026-06-28-fail-closed")
    );
    assert_ne!(active["tool_call"]["call_id"], json!("local-fail-closed"));
    assert!(
        active["tool_call"]["call_id"]
            .as_str()
            .expect("call id")
            .starts_with("registry_probe-fail-closed-")
    );
    assert!(
        active["round_id"]
            .as_str()
            .expect("round id")
            .starts_with("registry_probe-unsupported-live-surface-")
    );

    std::fs::remove_dir_all(root).expect("cleanup cli registry runtime boundary");
}

#[test]
fn registry_probe_fail_closed_rows_report_local_disk_and_global_truth() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-registry-local-truth");
    let home = root.join("home");
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"name":"harness-ultragoal","version":"0.0.0-test"}),
    );
    for (_, _, custom_agent_path) in reviewer_specs() {
        let source = root.join(custom_agent_path);
        let install = home
            .join(".codex/plugins/harness-ultragoal")
            .join(custom_agent_path);
        let cache = home
            .join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal/0.0.0-test")
            .join(custom_agent_path);
        let global = home.join(".codex/agents").join(
            Path::new(custom_agent_path)
                .file_name()
                .expect("agent file"),
        );
        for path in [&source, &install, &cache, &global] {
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(path, format!("name = \"{}\"\n", custom_agent_path)).expect("write");
        }
    }

    let rows = crate::cli::control::plane::registry::agent_types_for_home(&root, Some(home));
    assert!(rows.iter().all(|row| {
        row["disk_cache_synced"] == json!(true)
            && row["global_toml_present"] == json!(true)
            && row["exposed"] == json!(false)
    }));

    std::fs::remove_dir_all(root).expect("cleanup cli registry local truth");
}

fn live_registry_receipt(current: &str, raw_rel: &str, raw_digest: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.multi-agent-registry-exposure.v1",
        "generated_at": "2026-06-28T00:00:00Z",
        "captured_at": "2026-06-28T00:00:00Z",
        "status": "pass",
        "issuer": {"tool":"multi_agent_v1","authority":"tool_registry"},
        "tool_call": {
            "name": "multi_agent_v1.tool_registry",
            "call_id": "call-1",
            "arguments_digest": crate::self_tests::boundaries::support::sha('1')
        },
        "capture_method": "live_tool_registry_query",
        "boundary": {"account_id": "acct", "workspace_id": "workspace", "session_id": "session"},
        "source": "multi_agent_v1.tool_registry",
        "target_revision": {"kind":"package_digest","value":current},
        "claim_ceiling": "live_registry_reviewer_exposure_proven",
        "session_id": "session",
        "round_id": "round",
        "raw_observation": {"path": raw_rel, "digest": raw_digest},
        "agent_types": agent_types()
    })
}

fn raw_observation(current: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.registry-raw-observation.v1",
        "candidate_digest": current,
        "captured_at": "2026-06-28T00:00:00Z",
        "issuer": {"tool":"multi_agent_v1","authority":"tool_registry"},
        "tool_call": {
            "name": "multi_agent_v1.tool_registry",
            "call_id": "call-1",
            "arguments_digest": crate::self_tests::boundaries::support::sha('1')
        },
        "boundary": {"account_id": "acct", "workspace_id": "workspace", "session_id": "session"},
        "source": "multi_agent_v1.tool_registry",
        "registry_rows": agent_types()
    })
}

fn agent_types() -> Vec<Value> {
    reviewer_specs()
        .into_iter()
        .map(|(agent_type, persona, custom_agent_path)| {
            json!({
                "agent_type": agent_type,
                "persona": persona,
                "custom_agent_path": custom_agent_path,
                "disk_cache_synced": true,
                "global_toml_present": true,
                "exposed": true
            })
        })
        .collect()
}

fn reviewer_specs() -> Vec<(&'static str, &'static str, &'static str)> {
    [
        (
            "harness_contract_claim_falsifier",
            "contract_claim_falsifier",
            "custom-agents/harness-contract-claim-falsifier.toml",
        ),
        (
            "harness_orchestration_recovery_falsifier",
            "orchestration_recovery_falsifier",
            "custom-agents/harness-orchestration-recovery-falsifier.toml",
        ),
        (
            "harness_security_trust_boundary_falsifier",
            "security_trust_boundary_falsifier",
            "custom-agents/harness-security-trust-boundary-falsifier.toml",
        ),
        (
            "harness_product_simplicity_falsifier",
            "product_simplicity_falsifier",
            "custom-agents/harness-product-simplicity-falsifier.toml",
        ),
    ]
    .into_iter()
    .collect()
}
