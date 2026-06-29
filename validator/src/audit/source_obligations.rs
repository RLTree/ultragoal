use crate::json_boundary;
use serde_json::Value;
use std::path::Path;

const REQUIRED_OBLIGATIONS: &[&str] = &[
    "active-setup-to-idle-orchestration-thread-bound-heartbeat",
    "adversarial-packet-tampering-forged-proof-rejection",
    "agent-authored-source-tooling-docs-provenance",
    "agent-queryable-observability",
    "agent-remediating-validator-failures",
    "agent-session-telemetry-token-rate-limit",
    "architecture-dependency-topology",
    "authority-exhaustiveness-closed-enums-impossible-state-elimination",
    "authority-source-binding",
    "autonomy-loop-proof",
    "batch-fanout-custom-agent-job-worker-result-discipline",
    "behavior-example-coverage-coverage-anti-gaming",
    "capability-gap-extraction-harness-capability-promotion",
    "clean-checkout-command-discovery",
    "clean-room-rebuild-author-memory-independence",
    "cli-control-plane-authority",
    "cli-performance-latency-speed-iteration-fitness",
    "cli-self-law-compliance",
    "compact-agents-routed-standards",
    "conditional-observability-proof",
    "config-precedence-defaults-env-indirection",
    "connector-capability-discovery",
    "cross-artifact-consistency-solver-authority-graph-closure",
    "current-product-discovery-audit-quality-in-use-evidence",
    "derived-authority-recomputation",
    "derived-authority-recomputation-named-authority-fallback-refusal",
    "distinct-proof-surfaces-claim-ceilings",
    "distribution-sharing-surface-claim-separation",
    "execplan-no-handback-prototype-promotion-discard",
    "execplan-plain-language-expected-output-interface-completeness",
    "failure-remediation-quality-agent-actionable-output",
    "feedback-to-rule-promotion",
    "forward-only-state-transition-integrity-silent-reopen-prevention",
    "fresh-init-retrofit-mode-separation",
    "fresh-retrofit-repo-shape",
    "full-local-observability-stack-integration-non-opaque-failure",
    "generated-proof-artifact-provenance-anti-fabrication",
    "generated-ready-completion-receipts",
    "goal-contract-amendment-authority-required-claim-id-mapping",
    "green-path-adequacy-satisfiable-strictness",
    "guardrail-speed-isolation-cache-honesty",
    "historical-regression-corpus-session-review-signals",
    "human-audit-disposition-decomposition-judgment-claim-blocking",
    "instruction-precedence-nested-agents-routing",
    "issue-tracker-lifecycle-eligibility-terminal-state",
    "lane-worktree-isolation-cleanup",
    "live-beneficial-e2e",
    "memory-wiki-context-only",
    "namespace-progressive-disclosure",
    "validator-source-namespace-topology",
    "non-e2e-claim-ceiling-confidence-bounds",
    "offline-schema-catalog-resolver-portability",
    "one-command-fresh-environment-bootstrap-concurrent-resource-allocation",
    "orchestrator-state-machine",
    "plugin-bundled-component-graph-hook-app-mcp-safety",
    "plugin-flow-graph-package-dependency-closure-plugin-product-journey",
    "plugin-install-surface-metadata-cache-enable-state",
    "portable-non-prescriptive-adapter-implementation-choice",
    "privacy-raw-artifact-boundary",
    "product-cohesion-product-claims",
    "product-live-surface-receipts",
    "product-proof-joins-substitution-blocking",
    "product-strategy-positioning-research-eval-before-lane-planning",
    "product-success-binding-goal-lane-execplan-authority",
    "product-success-contract-initiation-authority",
    "product-success-contract-review-packet-team-skill-routing",
    "product-success-inspiration-source-provenance-disposition",
    "product-success-lifecycle-transitions-no-late-afterthought",
    "product-success-lineage-amendments-closed-product-claim-ids",
    "purpose-backed-active-files",
    "quality-score-taste-gates",
    "raw-private-artifact-handling-category-only-evidence",
    "repo-knowledge-index-core-beliefs",
    "restartable-execplans",
    "review-disagreement-override-judgment-boundary-governance",
    "review-feedback-disposition-same-round-satisfaction",
    "reviewer-to-gate-conversion",
    "rust-cache-no-cache-honesty",
    "rust-command-loop-authority",
    "rust-developer-experience-authority",
    "rust-memory-resource-discipline",
    "rust-toolchain-substrate-authority",
    "runtime-feasibility-cost-strict-gate-usability",
    "runtime-tool-identity",
    "scheduler-runner-tracker-boundaries",
    "schema-evolution-receipt-migration-stale-version-invalidation",
    "secret-token-boundaries",
    "semantic-domain-type-naming",
    "skill-catalog-context-budget-omission-warning",
    "skill-local-reference-closure",
    "skill-progressive-disclosure-metadata",
    "source-card-freshness-ceiling",
    "source-installed-cache-alignment",
    "source-obligation-parity-anti-bundling",
    "stable-identifier-normalization-collision",
    "standards-gardener-promotion",
    "subagent-custom-agent-sandbox-approval-inheritance",
    "subagent-orchestration-explicitness-token-model-cost-result-reconciliation",
    "target-repo-audit-capability",
    "targeted-refactor-debt-removal-standards-gardener-cadence",
    "template-generation-governance-template-creator-boundary",
    "third-party-dependency-legibility-typed-adapters",
    "total-authority-types-impossible-state-elimination",
    "transcript-quality-reuse-gates",
    "trust-boundary-abuse-path-failure-path-coverage",
    "typed-records-over-prose",
    "validator-theater-miswire-resistance",
    "value-adoption-continuance-business-mission-evidence-hierarchy",
    "workflow-template-parsing-rendering-reload",
    "workspace-artifact-cache-garbage-collection",
    "workspace-command-confinement-lifecycle-cleanup",
    "worktree-lane-owner-cost-policy",
];

const WEAK_DISPOSITION_TERMS: &[&str] = &[
    "manual audit",
    "human audit",
    "deterministic_with_human_audit",
    "partial",
    "backlog",
    "backlogged",
    "blocked",
    "reviewer_and_backlog",
    "reviewer-only",
    "future",
    "missing receipt",
    "future receipt",
    "future validator",
    "prose-only",
    "row-shape-only",
    "stale-source-backed",
    "not_refreshed",
];

pub fn failures(root: &Path) -> Vec<String> {
    match json_boundary::read_json(&root.join("docs/source-obligation-matrix.json")) {
        Ok(value) => {
            let mut failures = value_failures(&value);
            failures.extend(super::foundational_law_trace::failures(root, &value));
            failures.extend(super::law::family::aliases::failures(root));
            failures
        }
        Err(err) => vec![format!("docs/source-obligation-matrix.json: {err}")],
    }
}

pub fn value_failures(value: &Value) -> Vec<String> {
    let Some(rows) = value.get("obligations").and_then(Value::as_array) else {
        return vec!["source_obligation_matrix_missing_rows".to_string()];
    };
    let mut failures = REQUIRED_OBLIGATIONS
        .iter()
        .filter(|id| {
            !rows
                .iter()
                .any(|row| row.get("id").and_then(Value::as_str) == Some(**id))
        })
        .map(|id| {
            if *id == "namespace-progressive-disclosure" {
                "namespace_law_present_only_as_prose".to_string()
            } else {
                format!("{id}: source_obligation_missing_row")
            }
        })
        .collect::<Vec<_>>();
    failures.extend(rows.iter().filter_map(row_failure));
    failures
}

fn row_failure(row: &Value) -> Option<String> {
    let id = row.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let disposition = row
        .get("enforcement_disposition")
        .and_then(Value::as_str)
        .unwrap_or("");
    if disposition.is_empty() {
        return Some(format!("{id}: source_obligation_missing_disposition"));
    }
    if let Some(term) = weak_row_term(row) {
        return Some(format!("{id}: source_obligation_weak_disposition:{term}"));
    }
    if row
        .get("missing_validation_fixture_or_receipt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        return Some(format!("{id}: source_obligation_missing_gap_field"));
    }
    let tokens: &[&str] = match id {
        "derived-authority-recomputation" => &["deterministic", "canonical", "recomput", "digest"],
        "authority-source-binding" => &["authority", "fallback", "receipt", "claim"],
        "source-installed-cache-alignment" => &["source", "installed", "cache", "receipt"],
        "namespace-progressive-disclosure" => &["namespace", "progressive", "validator", "red"],
        "validator-source-namespace-topology" => {
            &["validator", "source", "topology", "typed", "red", "tamper"]
        }
        _ => return None,
    };
    required_token_row_failure(row, id, tokens)
}

fn weak_row_term(row: &Value) -> Option<&'static str> {
    let merged = [
        "enforcement_disposition",
        "coverage_status",
        "missing_validation_fixture_or_receipt",
        "claim_ceiling_impact",
    ]
    .iter()
    .filter_map(|key| row.get(*key).and_then(Value::as_str))
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    WEAK_DISPOSITION_TERMS
        .iter()
        .copied()
        .find(|term| merged.contains(term))
}

fn required_token_row_failure(row: &Value, id: &str, tokens: &[&str]) -> Option<String> {
    let merged = [
        "obligation",
        "package_surface",
        "enforcement_disposition",
        "coverage_status",
        "missing_validation_fixture_or_receipt",
        "claim_ceiling_impact",
    ]
    .iter()
    .filter_map(|key| row.get(*key).and_then(Value::as_str))
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    for token in tokens {
        if !merged.contains(token) {
            return Some(format!("{id}: source_obligation_missing_{token}"));
        }
    }
    None
}
