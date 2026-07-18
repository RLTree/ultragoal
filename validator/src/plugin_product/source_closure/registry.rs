use super::model::{BuildClosurePolicy, BuildInputKind, ClosureError, RequiredBuildInput};

/// Exact source-local closure before root extends it from fresh dep-info.
pub fn plugin_product_build_policy() -> Result<BuildClosurePolicy, ClosureError> {
    use BuildInputKind::{
        CargoLock, CargoManifest, DynamicInput, RuntimeAuthority, RustSource, VerifierInput,
    };
    let rows = [
        ("Cargo.lock", CargoLock),
        ("Cargo.toml", CargoManifest),
        ("validator/Cargo.toml", CargoManifest),
        ("docs/install-and-visibility.md", RuntimeAuthority),
        ("docs/plugin-resource-map.md", RuntimeAuthority),
        (
            "fixtures/plugin-product/journey-controls.tsv",
            RuntimeAuthority,
        ),
        (
            "fixtures/plugin-product/lifecycle-cases.json",
            RuntimeAuthority,
        ),
        (
            "fixtures/plugin-product/product-fitness-cases.json",
            RuntimeAuthority,
        ),
        (
            "fixtures/plugin-product/root-wiring-request.json",
            RuntimeAuthority,
        ),
        ("fixtures/plugin-product/route-cases.tsv", RuntimeAuthority),
        ("skills/diagnose-and-observe/SKILL.md", RuntimeAuthority),
        ("skills/goal-run/SKILL.md", RuntimeAuthority),
        ("skills/harness-ultragoal/SKILL.md", RuntimeAuthority),
        ("skills/improve-and-maintain/SKILL.md", RuntimeAuthority),
        ("skills/product-journey-review/SKILL.md", RuntimeAuthority),
        ("skills/prove/SKILL.md", RuntimeAuthority),
        ("skills/repository-fit/SKILL.md", RuntimeAuthority),
        ("skills/routine-work/SKILL.md", RuntimeAuthority),
        ("validator/src/plugin_product/mod.rs", RustSource),
        ("validator/src/plugin_product/journey_matrix.rs", RustSource),
        ("validator/src/plugin_product/lifecycle/mod.rs", RustSource),
        (
            "validator/src/plugin_product/lifecycle/execution.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/lifecycle/model.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/lifecycle/plan/mod.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/lifecycle/plan/transitions.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/product_fitness/mod.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/product_fitness/evidence.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/product_fitness/ladder.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/product_fitness/model.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/source_closure/mod.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/source_closure/filesystem.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/source_closure/format.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/source_closure/model.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/source_closure/policy.rs",
            RustSource,
        ),
        (
            "validator/src/plugin_product/source_closure/registry.rs",
            RustSource,
        ),
        (
            "validator/tests/plugin_product_contract/dependency_closure/mod.rs",
            VerifierInput,
        ),
        (
            "validator/tests/plugin_product_contract/host_truth_layers.rs",
            VerifierInput,
        ),
        (
            "validator/tests/plugin_product_contract/lifecycle_contract/mod.rs",
            VerifierInput,
        ),
        (
            "validator/tests/plugin_product_contract/main.rs",
            VerifierInput,
        ),
        (
            "validator/tests/plugin_product_contract/product_fitness_contract/mod.rs",
            VerifierInput,
        ),
        (
            "validator/tests/plugin_product_contract/route_contract.rs",
            VerifierInput,
        ),
        (
            "validator/tests/plugin_product_contract/source_contract/mod.rs",
            VerifierInput,
        ),
        (
            "validator/tests/plugin_product_contract/zero_write/mod.rs",
            VerifierInput,
        ),
        (
            "docs/ultragoal-successor-live/root-decisions/CANONICAL-PLUGIN-TRANSITIVE-PRODUCT-FITNESS-REWORK-020.json",
            DynamicInput,
        ),
        (
            "docs/ultragoal-successor-live/worker-results/CANONICAL-PLUGIN-DEPENDENCY-CLOSURE-018.json",
            DynamicInput,
        ),
        ("schemas/product-fitness-receipt.schema.json", DynamicInput),
        (
            "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/02-PLUGIN-PRODUCT-AND-JOURNEYS.md",
            DynamicInput,
        ),
        (
            "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/06-SECURITY-SUPPLY-AND-PRIVACY.md",
            DynamicInput,
        ),
        (
            "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CLAIM_REGISTRY.json",
            DynamicInput,
        ),
        (
            "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CONTRACT_MANIFEST.json",
            DynamicInput,
        ),
        (
            "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/IMPLEMENTATION_DEPENDENCY_GRAPH.json",
            DynamicInput,
        ),
        (
            "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/OPEN-DECISIONS.md",
            DynamicInput,
        ),
        (
            "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/REQUIREMENT_TRACE.json",
            DynamicInput,
        ),
        (
            "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/ULTRA-INPUT-MANIFEST.json",
            DynamicInput,
        ),
        (
            "validator/src/audit/product/fitness/evidence.rs",
            VerifierInput,
        ),
        ("validator/src/audit/product/fitness/mod.rs", VerifierInput),
        (
            "validator/src/audit/product/fitness/receipt/fields.rs",
            VerifierInput,
        ),
        (
            "validator/src/audit/product/fitness/receipt/mod.rs",
            VerifierInput,
        ),
        (
            "validator/src/audit/product/fitness/substitutions.rs",
            VerifierInput,
        ),
        ("validator/src/cli/successor/catalog.rs", RuntimeAuthority),
        ("validator/src/distribution/cache.rs", RustSource),
        ("validator/src/distribution/install.rs", RustSource),
        ("validator/src/distribution/mod.rs", RustSource),
        ("validator/src/distribution/model.rs", RustSource),
        ("validator/src/distribution/package/snapshot.rs", RustSource),
        ("validator/src/distribution/runtime_probe.rs", RustSource),
        ("validator/src/distribution/verify.rs", RustSource),
        ("validator/src/orchestration/model.rs", VerifierInput),
        ("validator/src/orchestration/worker.rs", VerifierInput),
        (
            "validator/src/review/round/product/fitness/criteria.rs",
            VerifierInput,
        ),
        (
            "validator/src/review/round/product/fitness/mod.rs",
            VerifierInput,
        ),
    ];
    BuildClosurePolicy::new(
        rows.into_iter()
            .map(|(path, kind)| RequiredBuildInput {
                path: path.to_owned(),
                kind,
            })
            .collect(),
    )
}
