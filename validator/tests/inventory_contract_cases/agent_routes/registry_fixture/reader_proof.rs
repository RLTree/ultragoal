pub(super) const READER_PROOF: &str =
    "docs/ultragoal-successor-live/worker-results/LEASE-N02-AGENT-READERS-002.json";
const BROAD_ROUTE: &str = "legacy-agents-to-current-discovery";

pub(super) type Case = (&'static str, &'static str);

pub(super) const CASES: [Case; 14] = [
    (
        "agents/contract-claim-falsifier.md",
        "AGENT:claim-falsifier",
    ),
    (
        "custom-agents/harness-contract-claim-falsifier.toml",
        "AGENT:claim-falsifier",
    ),
    (
        "agents/orchestration-recovery-falsifier.md",
        "AGENT:orchestration-recovery-reviewer",
    ),
    (
        "custom-agents/harness-orchestration-recovery-falsifier.toml",
        "AGENT:orchestration-recovery-reviewer",
    ),
    (
        "agents/security-trust-boundary-falsifier.md",
        "AGENT:security-reviewer",
    ),
    (
        "custom-agents/harness-security-trust-boundary-falsifier.toml",
        "AGENT:security-reviewer",
    ),
    (
        "agents/product-simplicity-falsifier.md",
        "AGENT:product-journey-reviewer",
    ),
    (
        "custom-agents/harness-product-simplicity-falsifier.toml",
        "AGENT:product-journey-reviewer",
    ),
    (
        "agents/material-review-scope-gatekeeper.md",
        "AGENT:product-journey-reviewer",
    ),
    (
        "custom-agents/harness-material-review-scope-gatekeeper.toml",
        "AGENT:product-journey-reviewer",
    ),
    ("agents/plugin-scout.md", "AGENT:repo-recon"),
    ("agents/standards-extractor.md", "AGENT:research-verifier"),
    (
        "custom-agents/harness-repo-initializer.toml",
        "PS-FIT-FRESH",
    ),
    (
        "custom-agents/harness-retrofit-planner.toml",
        "PS-FIT-RETROFIT",
    ),
];

const READER_SOURCES: &[&str] = &[
    "validator/src/agent_manifest.rs",
    "validator/src/agent_roles.rs",
    "validator/src/audit/contract.rs",
    "validator/src/audit/coverage/scope/roots.rs",
    "validator/src/audit/plugin/product/cohesion/manifest.rs",
    "validator/src/audit/plugin/product/cohesion/mod.rs",
    "validator/src/inventory/components.rs",
    "validator/src/inventory/registry/topology.rs",
    "validator/src/package/inventory/mod.rs",
    "validator/src/package/inventory/anchored/json.rs",
    "validator/src/package/inventory/anchored/mod.rs",
    "validator/src/package/inventory/anchored/session.rs",
    "validator/src/package/inventory/anchored/snapshot.rs",
    "validator/src/package/inventory/anchored/sys.rs",
    "validator/src/package/inventory/closure/mod.rs",
    "validator/src/package/inventory/closure/package_entries.rs",
    "validator/src/package/inventory/generated_disposition/anchored.rs",
    "validator/src/package/inventory/generated_disposition/mod.rs",
    "validator/src/package/inventory/generated_disposition/schema.rs",
    "validator/src/package/inventory/generated_disposition/unique_json.rs",
    "validator/src/package/inventory/payload.rs",
    "validator/src/package/inventory/snapshot/capture.rs",
    "validator/src/claim_semantics/plugin_policy/mod.rs",
    "validator/src/claim_semantics/coverage/receipt/exclusions.rs",
    "validator/src/claim_semantics/retired_reviewer_policy.rs",
    "validator/src/cli/control/plane/registry/mod.rs",
    "validator/src/cli/control/plane/registry/agent_rows.rs",
    "validator/src/cli/live_loop/surfaces/input_spec/path_rules.rs",
    "validator/src/audit/plugin/registry/mod.rs",
    "validator/src/audit/plugin/registry/live/mod.rs",
    "validator/src/audit/plugin/registry/live/raw.rs",
    "validator/src/audit/plugin/registry/live/reviewers.rs",
    "validator/src/review/round/config.rs",
    "validator/src/review/round/personas.rs",
    "validator/src/review/round/registry.rs",
    "validator/src/review/round/registry/reader.rs",
    "validator/src/review/round/registry/reader/json.rs",
    "validator/src/review/round/registry/reader/path.rs",
    "validator/src/review/round/registry/semantics.rs",
    "validator/src/review/round/registry/validation.rs",
    "validator/src/inventory/compatibility/agent/specs.rs",
    "validator/src/inventory/compatibility/agent/witness.rs",
    "validator/src/inventory/compatibility/reader_witness.rs",
    "validator/src/inventory/compatibility/reader_witness/scan.rs",
    "validator/src/inventory/compatibility/reader_witness/scan/literals.rs",
    "validator/src/inventory/compatibility/reader_witness/scan/unicode.rs",
    "validator/src/inventory/compatibility/reader_witness_specs.rs",
    "validator/src/inventory/walk.rs",
    "validator/src/orchestration/artifact.rs",
    "validator/src/orchestration/scope_policy.rs",
    "validator/src/inventory/discovery.rs",
    "validator/src/inventory/legacy/matchers.rs",
    "validator/src/inventory/legacy/scope.rs",
    "validator/src/inventory/agent/manifest_digests.rs",
    "validator/src/inventory/agent/package_digests.rs",
    "validator/src/inventory/plugin_manifest_hooks.rs",
    "validator/src/inventory/validate.rs",
    "validator/src/lib.rs",
    "validator/src/package/mod.rs",
    "validator/src/red/fixture/package.rs",
    "validator/src/review/materiality.rs",
    "validator/src/review/materiality/registry.rs",
    "validator/src/review/round/report.rs",
    "validator/src/schema_catalog/fixture_schema_rules.rs",
    "validator/src/schema_catalog/mod.rs",
    "validator/src/schema_catalog/schema/patterns.rs",
    "validator/src/target_repo/baseline.rs",
    "validator/src/target_repo/baseline_mode.rs",
];

const AGENT_MANIFESTS: &[&str] = &[
    ".codex/agents/claim-falsifier.toml",
    ".codex/agents/orchestration-recovery-reviewer.toml",
    ".codex/agents/product-journey-reviewer.toml",
    ".codex/agents/repo-recon.toml",
    ".codex/agents/research-verifier.toml",
    ".codex/agents/security-reviewer.toml",
];

pub(super) fn route_id(case: Case) -> String {
    let legacy = std::path::Path::new(case.0)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();
    let target = case
        .1
        .strip_prefix("AGENT:")
        .or_else(|| case.1.strip_prefix("PS-"))
        .unwrap()
        .to_ascii_lowercase();
    format!("agent-{legacy}-to-{target}")
}

pub(super) fn target_path(case: Case) -> String {
    match case.1.strip_prefix("AGENT:") {
        Some(role) => format!(".codex/agents/{role}.toml"),
        None => format!(
            "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/PRODUCT_SURFACE_INVENTORY.json#/product-surface-definition/{}",
            case.1
        ),
    }
}

fn transition(case: Case) -> Value {
    json!({
        "compatibility_behavior": "not-applicable",
        "compatibility_boundary": "adopted",
        "replacement_state": "candidate-required",
        "active_reader_writer_state": "none-verified",
        "observed_authority_state": "context-only",
        "equivalence_proof": "not-applicable",
        "physical_cleanup_state": "preserve",
        "proof_refs": [case.0, target_path(case), READER_PROOF]
    })
}

fn route(case: Case) -> Value {
    json!({
        "route_id": route_id(case),
        "match": {"stable_id": format!("LEGACY-AGENT:{}", case.0)},
        "canonical_target": case.1,
        "intended_disposition": "non-authoritative",
        "transition": transition(case)
    })
}

pub(super) fn registry(repo: &TestRepo) -> Value {
    serde_json::from_slice(&fs::read(repo.root.join("migration/authority-routes.json")).unwrap())
        .unwrap()
}
