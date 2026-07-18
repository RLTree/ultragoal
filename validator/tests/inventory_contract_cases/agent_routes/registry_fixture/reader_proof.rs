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
