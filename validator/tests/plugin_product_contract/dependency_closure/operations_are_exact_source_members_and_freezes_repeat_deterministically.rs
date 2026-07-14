#[test]
fn operations_are_exact_source_members_and_freezes_repeat_deterministically() {
    let closure = closure();
    let envelope = envelope();
    assert_eq!(closure.schema_version, "HarnessPluginCandidateClosure-v1");
    assert_eq!(
        closure.artifact_set_formula,
        "Sort paths; concatenate each path, one TAB byte, lowercase SHA-256 without prefix, and one LF byte; SHA-256 the result."
    );
    assert!(closure.worker_result_self_exclusion.contains("excluded"));
    let source = [
        read("README.md"),
        read("docs/install-and-visibility.md"),
        read("docs/plugin-resource-map.md"),
        read("skills/diagnose-and-observe/SKILL.md"),
        read("skills/goal-run/SKILL.md"),
        read("skills/harness-ultragoal/SKILL.md"),
        read("skills/improve-and-maintain/SKILL.md"),
        read("skills/product-journey-review/SKILL.md"),
        read("skills/prove/SKILL.md"),
        read("skills/repository-fit/SKILL.md"),
        read("skills/routine-work/SKILL.md"),
    ]
    .join("\n");
    let ids = closure
        .executable_read_operations
        .iter()
        .map(|operation| {
            assert!(!operation.args.is_empty());
            assert!(
                source.contains(&operation.source_marker),
                "{}",
                operation.id
            );
            operation.id.as_str()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 16);
    let first = aggregate(&closure, &envelope, None, &BTreeMap::new());
    let second = aggregate(&closure, &envelope, None, &BTreeMap::new());
    assert_eq!(first, second);
    let worker = aggregate(
        &closure,
        &envelope,
        Some(Authority::WorkerOwned),
        &BTreeMap::new(),
    );
    assert_ne!(worker, first);
}

#[test]
fn prior_worker_result_remains_typed_but_is_stale_after_lifecycle_correction() {
    let bytes = std::fs::read(root().join(RESULT_PATH))
        .unwrap_or_else(|error| panic!("emitted WorkerResult-v1 unavailable: {error}"));
    let result = WorkerResultV1::parse_json(&bytes)
        .unwrap_or_else(|error| panic!("WorkerResult-v1 invalid: {error}"));
    let envelope = envelope();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .unwrap_or_else(|error| panic!("WorkerResult-v1 lease binding invalid: {error}"));
    let verification = ArtifactWorkspace::new(root()).unwrap().verify(
        &result,
        &envelope.lease,
        &envelope.work_package,
    );
    assert!(
        verification.is_err(),
        "superseded candidate must not verify live"
    );
    assert_eq!(
        result.dependency_nodes,
        ["plugin-product-dependency-closure-rework"]
    );
    assert_eq!(
        result.candidate_identity["candidate_id"],
        envelope.issued_from_live_context.candidate_id
    );
    assert_eq!(
        result.candidate_identity["context_id"],
        envelope.issued_from_live_context.context_id
    );
}

const LIFECYCLE_ENVELOPE_PATH: &str =
    "docs/ultragoal-successor-live/work-packages/PLUGIN-LIFECYCLE-PRODUCT-CORRECTION-024.json";
const LIFECYCLE_RESULT_PATH: &str =
    "docs/ultragoal-successor-live/worker-results/PLUGIN-LIFECYCLE-PRODUCT-CORRECTION-024.json";
const LIFECYCLE_ENVELOPE_SHA256: &str =
    "sha256:02c94732f55382b848e11385c962e04983b8881805a0fdd3af45c3e3593717b5";
const LIFECYCLE_AUTH_ENVELOPE_PATH: &str = "docs/ultragoal-successor-live/work-packages/PLUGIN-LIFECYCLE-PLAN-AUTHORIZATION-CORRECTION-031.json";
const LIFECYCLE_AUTH_RESULT_PATH: &str = "docs/ultragoal-successor-live/worker-results/PLUGIN-LIFECYCLE-PLAN-AUTHORIZATION-CORRECTION-031.json";
const LIFECYCLE_AUTH_ENVELOPE_SHA256: &str =
    "sha256:6c65470208eda2e47fc9dd9c2aed8127fc3a4921a1b7c7f75da91afa04ffe544";
const LIFECYCLE_RECOVERY_ENVELOPE_PATH: &str = "docs/ultragoal-successor-live/work-packages/PLUGIN-LIFECYCLE-RECOVERY-ACTIVATION-CORRECTION-035.json";
const LIFECYCLE_RECOVERY_RESULT_PATH: &str = "docs/ultragoal-successor-live/worker-results/PLUGIN-LIFECYCLE-RECOVERY-ACTIVATION-CORRECTION-035.json";
const LIFECYCLE_RECOVERY_ENVELOPE_SHA256: &str =
    "sha256:45c2e2ce138a3b2d5e97e5cadeae10e8bcd5fd5304c9eda038630824bf5e410d";
const LIFECYCLE_READ_FAILURE_ENVELOPE_PATH: &str = "docs/ultragoal-successor-live/work-packages/PLUGIN-LIFECYCLE-READ-FAILURE-GUARD-CORRECTION-039.json";
const LIFECYCLE_READ_FAILURE_RESULT_PATH: &str = "docs/ultragoal-successor-live/worker-results/PLUGIN-LIFECYCLE-READ-FAILURE-GUARD-CORRECTION-039.json";
const LIFECYCLE_READ_FAILURE_ENVELOPE_SHA256: &str =
    "sha256:07ca18128ef998fb7645c4f2f3fa8bd8b9904a8a34146198442e6f3ccfb1be8e";
const WORKER_NO_CLAIM: &str = "This worker does not claim readiness, release, or completion.";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LifecycleRootEnvelope {
    schema_version: String,
    issued_from_live_context: IssuedContext,
    work_package: WorkPackage,
    lease: LeaseSpec,
    prohibited_surfaces: Vec<String>,
    root_validation_obligations: Vec<String>,
    no_claim_statement: String,
}

#[test]
fn exact_lifecycle_envelope_and_lease_revalidate_before_candidate_use() {
    let bytes = std::fs::read(root().join(LIFECYCLE_ENVELOPE_PATH)).unwrap();
    assert_eq!(
        format!("sha256:{}", digest(&bytes)),
        LIFECYCLE_ENVELOPE_SHA256
    );
    let envelope: LifecycleRootEnvelope = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(envelope.schema_version, "RootIssuedWorkEnvelope-v1");
    envelope.work_package.validate().unwrap();
    let policy = ScopePolicy {
        allowed_read_paths: envelope.work_package.read_paths.clone(),
        allowed_paths: envelope.work_package.owned_scope.paths.clone(),
        allowed_semantic_prefixes: BTreeSet::from(["plugin_product".to_owned()]),
        allowed_generated_outputs: envelope.work_package.owned_scope.generated_outputs.clone(),
        allowed_fixtures: envelope.work_package.owned_scope.fixtures.clone(),
        allowed_effects: envelope.work_package.owned_scope.effects.clone(),
        root_only_paths: BTreeSet::from([
            CanonicalPath::parse(".codex").unwrap(),
            CanonicalPath::parse(".agents").unwrap(),
            CanonicalPath::parse(".codex-plugin/plugin.json").unwrap(),
            CanonicalPath::parse("plugin-manifest-draft.json").unwrap(),
            CanonicalPath::parse("Cargo.toml").unwrap(),
            CanonicalPath::parse("Cargo.lock").unwrap(),
            CanonicalPath::parse("validator/src/lib.rs").unwrap(),
        ]),
        root_only_semantic_prefixes: BTreeSet::from(["root".to_owned()]),
        root_only_effect_classes: BTreeSet::from([
            EffectClass::Destructive,
            EffectClass::RootAuthority,
            EffectClass::ExternalWrite,
            EffectClass::Network,
        ]),
    };
    policy.validate().unwrap();
    envelope.lease.validate(&policy).unwrap();
    assert_eq!(
        envelope.lease.owner.as_str(),
        "/root/plugin_lifecycle_product_engineer"
    );
    assert_eq!(
        envelope.issued_from_live_context.context_id,
        envelope.lease.binding.context_id
    );
    assert_eq!(
        envelope.issued_from_live_context.candidate_id,
        envelope.lease.binding.candidate_id
    );
    assert!(!envelope.issued_from_live_context.head_commit.is_empty());
    assert!(!envelope.issued_from_live_context.head_tree.is_empty());
    assert!(!envelope.issued_from_live_context.branch.is_empty());
    assert!(!envelope.issued_from_live_context.dirty);
    assert_eq!(envelope.prohibited_surfaces.len(), 6);
    assert_eq!(envelope.root_validation_obligations.len(), 3);
    assert!(envelope.no_claim_statement.contains("raises no source"));
}

struct ClosureTempRoot(std::path::PathBuf);
