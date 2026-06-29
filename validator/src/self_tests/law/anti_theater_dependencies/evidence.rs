use serde_json::json;

pub(super) fn items(current: &str) -> Vec<serde_json::Value> {
    paths()
        .into_iter()
        .map(|(label, path)| {
            json!({
                "label": label,
                "path": path,
                "exists": true,
                "digest": current,
                "schema": "test-green-receipt.v1",
                "status": "pass",
                "candidate_digest": current,
                "same_candidate": true,
                "failures": []
            })
        })
        .collect()
}

fn paths() -> [(&'static str, &'static str); 27] {
    [
        (
            "source_audit",
            "validation_artifacts/ultragoal-audit/validator-receipt.json",
        ),
        (
            "red_fixture_report",
            "validation_artifacts/ultragoal-audit/red-fixture-report.json",
        ),
        (
            "coverage",
            "validation_artifacts/coverage/coverage-receipt.json",
        ),
        (
            "cli_performance",
            "validation_artifacts/cli/performance-receipt.json",
        ),
        (
            "final_packet",
            "validation_artifacts/review/final-packet-proof.json",
        ),
        (
            "registry_exposure",
            "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        ),
        (
            "fit_repo",
            "validation_artifacts/harness/fit-repo-receipt.json",
        ),
        (
            "product_fitness",
            "validation_artifacts/harness/product-fitness-receipt.json",
        ),
        (
            "product_journey",
            "validation_artifacts/harness/plugin-product-journey-receipt.json",
        ),
        (
            "standards_gardener",
            "validation_artifacts/standards-gardener/current-standards-gardening-receipt.json",
        ),
        (
            "install_audit",
            "validation_artifacts/cli/install-audit-receipt.json",
        ),
        (
            "cache_audit",
            "validation_artifacts/cli/cache-audit-receipt.json",
        ),
        (
            "rust_toolchain",
            "validation_artifacts/rust/toolchain-receipt.json",
        ),
        ("rust_fast", "validation_artifacts/rust/fast-receipt.json"),
        (
            "rust_standard",
            "validation_artifacts/rust/standard-receipt.json",
        ),
        (
            "rust_release",
            "validation_artifacts/rust/release-receipt.json",
        ),
        (
            "rust_clean_proof",
            "validation_artifacts/rust/clean-proof-receipt.json",
        ),
        ("rust_watch", "validation_artifacts/rust/watch-receipt.json"),
        (
            "rust_memory",
            "validation_artifacts/rust/memory-receipt.json",
        ),
        (
            "rust_dependency",
            "validation_artifacts/rust/dependency-receipt.json",
        ),
        (
            "rust_coverage",
            "validation_artifacts/rust/coverage-receipt.json",
        ),
        (
            "rust_workspace_topology",
            "validation_artifacts/rust/workspace-topology-receipt.json",
        ),
        ("gc_plan", "validation_artifacts/gc/plan-receipt.json"),
        ("gc_dry_run", "validation_artifacts/gc/dry-run-receipt.json"),
        ("gc_apply", "validation_artifacts/gc/apply-receipt.json"),
        ("gc_verify", "validation_artifacts/gc/verify-receipt.json"),
        (
            "transactional_finalization",
            "validation_artifacts/cli/transactional-finalization-receipt.json",
        ),
    ]
}
