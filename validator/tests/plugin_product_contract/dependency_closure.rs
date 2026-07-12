use super::{read, root};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use ultragoal::orchestration::{
    ArtifactWorkspace, CanonicalPath, EffectClass, LeaseSpec, ScopePolicy, WorkPackage,
    WorkerResultV1,
};

const ENVELOPE_PATH: &str =
    "docs/ultragoal-successor-live/work-packages/CANONICAL-PLUGIN-DEPENDENCY-CLOSURE-018.json";
const RESULT_PATH: &str =
    "docs/ultragoal-successor-live/worker-results/CANONICAL-PLUGIN-DEPENDENCY-CLOSURE-018.json";
const ENVELOPE_SHA256: &str =
    "sha256:9cd652e0eb96f1a0d51a77385728c46a87487e5d8b0b65f7062e9d1b16035d7e";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootIssuedEnvelope {
    schema_version: String,
    issued_from_live_context: IssuedContext,
    work_package: WorkPackage,
    lease: LeaseSpec,
    protected_root_inputs: Vec<ProtectedRootInput>,
    root_validation_obligations: Vec<String>,
    no_claim_statement: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IssuedContext {
    context_id: String,
    candidate_id: String,
    head_commit: String,
    head_tree: String,
    branch: String,
    dirty: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectedRootInput {
    path: String,
    #[serde(default)]
    precondition: Option<String>,
    sha256: String,
    #[serde(default)]
    byte_length: Option<u64>,
    #[serde(default)]
    line_count: Option<u64>,
    worker_access: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateClosure {
    schema_version: String,
    artifact_set_formula: String,
    worker_result_self_exclusion: String,
    members: Vec<Member>,
    executable_read_operations: Vec<Operation>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    path: String,
    authority: Authority,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
enum Authority {
    WorkerOwned,
    RootRead,
    ProtectedRootMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    id: String,
    args: Vec<String>,
    source_marker: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DescriptorBinding {
    name: String,
    path: String,
    sha256: String,
    byte_length: u64,
    line_count: u64,
}

#[derive(Debug, Eq, PartialEq)]
enum ClosureError {
    Conflict,
    Missing,
    RootMetadataMismatch,
    Unknown,
}

fn envelope() -> RootIssuedEnvelope {
    serde_json::from_str(&read(ENVELOPE_PATH)).expect("typed root-issued envelope")
}

fn root_request() -> Value {
    serde_json::from_str(&read("fixtures/plugin-product/root-wiring-request.json"))
        .expect("root wiring request")
}

fn closure() -> CandidateClosure {
    serde_json::from_value(root_request()["candidate_closure"].clone())
        .expect("typed candidate closure")
}

fn expected_members() -> BTreeMap<&'static str, Authority> {
    BTreeMap::from([
        (
            ".agents/plugins/marketplace.json",
            Authority::ProtectedRootMetadata,
        ),
        (".codex-plugin/plugin.json", Authority::RootRead),
        (
            ".codex/agents/claim-falsifier.toml",
            Authority::ProtectedRootMetadata,
        ),
        (
            ".codex/agents/orchestration-recovery-reviewer.toml",
            Authority::ProtectedRootMetadata,
        ),
        (
            ".codex/agents/product-journey-reviewer.toml",
            Authority::ProtectedRootMetadata,
        ),
        (
            ".codex/agents/repo-recon.toml",
            Authority::ProtectedRootMetadata,
        ),
        (
            ".codex/agents/research-verifier.toml",
            Authority::ProtectedRootMetadata,
        ),
        (
            ".codex/agents/security-reviewer.toml",
            Authority::ProtectedRootMetadata,
        ),
        ("README.md", Authority::WorkerOwned),
        ("docs/install-and-visibility.md", Authority::WorkerOwned),
        ("docs/plugin-resource-map.md", Authority::WorkerOwned),
        (
            "fixtures/plugin-product/journey-controls.tsv",
            Authority::WorkerOwned,
        ),
        (
            "fixtures/plugin-product/root-wiring-request.json",
            Authority::WorkerOwned,
        ),
        (
            "fixtures/plugin-product/route-cases.tsv",
            Authority::WorkerOwned,
        ),
        ("plugin-manifest-draft.json", Authority::RootRead),
        (
            "skills/diagnose-and-observe/SKILL.md",
            Authority::WorkerOwned,
        ),
        ("skills/goal-run/SKILL.md", Authority::WorkerOwned),
        ("skills/harness-ultragoal/SKILL.md", Authority::WorkerOwned),
        (
            "skills/improve-and-maintain/SKILL.md",
            Authority::WorkerOwned,
        ),
        (
            "skills/product-journey-review/SKILL.md",
            Authority::WorkerOwned,
        ),
        ("skills/prove/SKILL.md", Authority::WorkerOwned),
        ("skills/repository-fit/SKILL.md", Authority::WorkerOwned),
        ("skills/routine-work/SKILL.md", Authority::WorkerOwned),
        (
            "validator/src/cli/successor/catalog.rs",
            Authority::RootRead,
        ),
        (
            "validator/tests/plugin_product_contract/dependency_closure.rs",
            Authority::WorkerOwned,
        ),
        (
            "validator/tests/plugin_product_contract/main.rs",
            Authority::WorkerOwned,
        ),
        (
            "validator/tests/plugin_product_contract/route_contract.rs",
            Authority::WorkerOwned,
        ),
        (
            "validator/tests/plugin_product_contract/source_contract.rs",
            Authority::WorkerOwned,
        ),
        (
            "validator/tests/plugin_product_contract/zero_write.rs",
            Authority::WorkerOwned,
        ),
    ])
}

fn conflicts(left: &str, right: &str) -> bool {
    let left = left.to_ascii_lowercase();
    let right = right.to_ascii_lowercase();
    left == right
        || left
            .strip_prefix(&right)
            .is_some_and(|rest| rest.starts_with('/'))
        || right
            .strip_prefix(&left)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn validate_membership(value: &CandidateClosure) -> Result<(), ClosureError> {
    for (index, member) in value.members.iter().enumerate() {
        if value.members[index + 1..]
            .iter()
            .any(|other| conflicts(&member.path, &other.path))
        {
            return Err(ClosureError::Conflict);
        }
    }
    let expected = expected_members();
    for member in &value.members {
        if expected.get(member.path.as_str()) != Some(&member.authority) {
            return Err(ClosureError::Unknown);
        }
    }
    if value.members.len() != expected.len() {
        return Err(ClosureError::Missing);
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn aggregate(
    value: &CandidateClosure,
    envelope: &RootIssuedEnvelope,
    authority: Option<Authority>,
    overrides: &BTreeMap<String, Vec<u8>>,
) -> String {
    let mut rows = value
        .members
        .iter()
        .filter(|member| authority.is_none_or(|expected| member.authority == expected))
        .map(|member| {
            let sha256 = if member.authority == Authority::ProtectedRootMetadata {
                envelope
                    .protected_root_inputs
                    .iter()
                    .find(|input| input.path == member.path)
                    .expect("protected metadata member")
                    .sha256
                    .trim_start_matches("sha256:")
                    .to_owned()
            } else {
                let bytes = overrides
                    .get(&member.path)
                    .cloned()
                    .unwrap_or_else(|| std::fs::read(root().join(&member.path)).unwrap());
                digest(&bytes)
            };
            (member.path.clone(), sha256)
        })
        .collect::<Vec<_>>();
    rows.sort();
    let mut freeze = Vec::new();
    for (path, sha256) in rows {
        freeze.extend_from_slice(path.as_bytes());
        freeze.push(b'\t');
        freeze.extend_from_slice(sha256.as_bytes());
        freeze.push(b'\n');
    }
    format!("sha256:{}", digest(&freeze))
}

fn validate_root_metadata(
    value: &CandidateClosure,
    envelope: &RootIssuedEnvelope,
) -> Result<(), ClosureError> {
    let request = root_request();
    let bindings: Vec<DescriptorBinding> =
        serde_json::from_value(request["descriptor_bindings"].clone()).unwrap();
    for input in &envelope.protected_root_inputs {
        if input.worker_access != "prohibited"
            || !value.members.iter().any(|member| {
                member.path == input.path && member.authority == Authority::ProtectedRootMetadata
            })
        {
            return Err(ClosureError::RootMetadataMismatch);
        }
        if input.path.starts_with(".codex/agents/") {
            let Some(binding) = bindings.iter().find(|binding| binding.path == input.path) else {
                return Err(ClosureError::RootMetadataMismatch);
            };
            if binding.name.is_empty()
                || binding.sha256.trim_start_matches("sha256:") != input.sha256
                || Some(binding.byte_length) != input.byte_length
                || Some(binding.line_count) != input.line_count
            {
                return Err(ClosureError::RootMetadataMismatch);
            }
        } else if input.path == ".agents/plugins/marketplace.json"
            && (input.precondition.as_deref() != Some("path_absent")
                || request["changes"][2]["precondition"] != "path_absent"
                || request["changes"][2]["preimage_sha256"]
                    .as_str()
                    .map(|value| value.trim_start_matches("sha256:"))
                    != Some(input.sha256.as_str()))
        {
            return Err(ClosureError::RootMetadataMismatch);
        }
    }
    Ok(())
}

#[test]
fn exact_root_envelope_parses_and_validates_without_protected_reads() {
    let bytes = std::fs::read(root().join(ENVELOPE_PATH)).unwrap();
    assert_eq!(format!("sha256:{}", digest(&bytes)), ENVELOPE_SHA256);
    let envelope: RootIssuedEnvelope = serde_json::from_slice(&bytes).unwrap();
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
            CanonicalPath::parse("plugin-manifest-draft.json").unwrap(),
        ]),
        root_only_semantic_prefixes: BTreeSet::from(["root".to_owned()]),
        root_only_effect_classes: BTreeSet::from([
            EffectClass::Destructive,
            EffectClass::RootAuthority,
        ]),
    };
    policy.validate().unwrap();
    envelope.lease.validate(&policy).unwrap();
    assert_eq!(envelope.lease.node_id, envelope.work_package.node_id);
    assert_eq!(
        envelope.lease.owned_scope,
        envelope.work_package.owned_scope
    );
    assert_eq!(
        envelope
            .lease
            .prerequisite_evidence
            .dependency_nodes
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["plugin-product-dependency-closure-rework"]
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
    assert!(envelope.issued_from_live_context.dirty);
    assert_eq!(envelope.protected_root_inputs.len(), 7);
    assert_eq!(envelope.root_validation_obligations.len(), 4);
    assert!(
        envelope
            .no_claim_statement
            .contains("grants no root authority")
    );
    assert!(
        envelope
            .lease
            .read_paths
            .iter()
            .all(|path| { !matches!(path.as_str().split('/').next(), Some(".codex" | ".agents")) })
    );
}

#[test]
fn typed_envelope_and_closure_reject_unknown_fields() {
    let mut envelope_value: Value = serde_json::from_str(&read(ENVELOPE_PATH)).unwrap();
    envelope_value["unknown"] = Value::Bool(true);
    assert!(serde_json::from_value::<RootIssuedEnvelope>(envelope_value).is_err());
    let mut closure_value = root_request()["candidate_closure"].clone();
    closure_value["members"][0]["unknown"] = Value::Bool(true);
    assert!(serde_json::from_value::<CandidateClosure>(closure_value).is_err());
}

#[test]
fn candidate_membership_unknown_missing_and_conflicts_fail_closed() {
    let live = closure();
    validate_membership(&live).unwrap();
    let mut unknown = live.clone();
    unknown.members[0].path = "unknown/product-input".to_owned();
    assert_eq!(validate_membership(&unknown), Err(ClosureError::Unknown));
    let mut missing = live.clone();
    missing.members.pop();
    assert_eq!(validate_membership(&missing), Err(ClosureError::Missing));
    let mut conflict = live;
    conflict.members.push(Member {
        path: "README.md/alias".to_owned(),
        authority: Authority::WorkerOwned,
    });
    assert_eq!(validate_membership(&conflict), Err(ClosureError::Conflict));
}

#[test]
fn catalog_and_protected_metadata_mutations_invalidate_the_full_extension() {
    let closure = closure();
    let envelope = envelope();
    validate_root_metadata(&closure, &envelope).unwrap();
    let live = aggregate(&closure, &envelope, None, &BTreeMap::new());
    let mut catalog_override = BTreeMap::new();
    let mut catalog = std::fs::read(root().join("validator/src/cli/successor/catalog.rs")).unwrap();
    catalog.extend_from_slice(b"\n// semantic mutation\n");
    catalog_override.insert("validator/src/cli/successor/catalog.rs".to_owned(), catalog);
    assert_ne!(
        aggregate(&closure, &envelope, None, &catalog_override),
        live
    );
    let mut metadata_mutation = envelope.clone();
    metadata_mutation.protected_root_inputs[0].sha256 = "0".repeat(64);
    assert_ne!(
        aggregate(&closure, &metadata_mutation, None, &BTreeMap::new()),
        live
    );
    assert_eq!(
        validate_root_metadata(&closure, &metadata_mutation),
        Err(ClosureError::RootMetadataMismatch)
    );
}

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
    let bytes = std::fs::read(root().join(RESULT_PATH)).expect("emitted WorkerResult-v1");
    let result = WorkerResultV1::parse_json(&bytes).expect("exact WorkerResult-v1 parse");
    let envelope = envelope();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .expect("lease-bound WorkerResult-v1");
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

impl ClosureTempRoot {
    fn new() -> Self {
        static NEXT_CLOSURE_ROOT: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "hul-build-closure-{}-{}",
            std::process::id(),
            NEXT_CLOSURE_ROOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("src")).unwrap();
        std::fs::create_dir_all(path.join("target")).unwrap();
        std::fs::write(path.join("Cargo.toml"), b"[package]\nname='probe'\n").unwrap();
        std::fs::write(path.join("Cargo.lock"), b"version = 4\n").unwrap();
        std::fs::write(path.join("src/lib.rs"), b"pub fn probe() {}\n").unwrap();
        std::fs::write(path.join("runtime.json"), b"{\"runtime\":true}\n").unwrap();
        std::fs::write(path.join("verifier.rs"), b"pub fn verify() {}\n").unwrap();
        let dep_info = format!("target/probe: {}\n", path.join("src/lib.rs").display());
        std::fs::write(path.join("target/probe.d"), dep_info).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for ClosureTempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn build_policy(
    root: &std::path::Path,
) -> super::plugin_product::source_closure::BuildClosurePolicy {
    use super::plugin_product::source_closure::{
        BuildClosurePolicy, BuildInputKind, RequiredBuildInput,
    };
    BuildClosurePolicy::from_dep_info(
        root,
        &["target/probe.d".to_owned()],
        vec![
            RequiredBuildInput {
                path: "Cargo.lock".to_owned(),
                kind: BuildInputKind::CargoLock,
            },
            RequiredBuildInput {
                path: "Cargo.toml".to_owned(),
                kind: BuildInputKind::CargoManifest,
            },
            RequiredBuildInput {
                path: "runtime.json".to_owned(),
                kind: BuildInputKind::RuntimeAuthority,
            },
            RequiredBuildInput {
                path: "verifier.rs".to_owned(),
                kind: BuildInputKind::VerifierInput,
            },
        ],
    )
    .unwrap()
}

#[test]
fn build_closure_captures_dep_source_runtime_and_verifier_inputs_deterministically() {
    use super::plugin_product::source_closure::BuildClosureV1;
    let root = ClosureTempRoot::new();
    let policy = build_policy(root.path());
    let first = BuildClosureV1::capture(root.path(), &policy).unwrap();
    let second = BuildClosureV1::capture(root.path(), &policy).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.schema_version, "BuildClosure-v1");
    assert!(first.final_session_revalidated);
    assert_eq!(first.rows.len(), 6);
    first.verify(root.path(), &policy).unwrap();
}

#[test]
fn build_closure_rejects_unknown_missing_duplicate_and_alias_rows() {
    use super::plugin_product::source_closure::{
        BuildClosurePolicy, BuildClosureV1, BuildInputKind, ClosureError, RequiredBuildInput,
    };
    let root = ClosureTempRoot::new();
    let policy = build_policy(root.path());
    let live = BuildClosureV1::capture(root.path(), &policy).unwrap();

    let mut unknown = live.clone();
    unknown.rows[0].path = "unknown.rs".to_owned();
    assert_eq!(
        unknown.verify(root.path(), &policy),
        Err(ClosureError::UnknownRow)
    );
    let mut missing = live;
    missing.rows.pop();
    assert_eq!(
        missing.verify(root.path(), &policy),
        Err(ClosureError::UnknownRow)
    );
    assert_eq!(
        BuildClosurePolicy::new(vec![
            RequiredBuildInput {
                path: "src/lib.rs".to_owned(),
                kind: BuildInputKind::RustSource,
            },
            RequiredBuildInput {
                path: "SRC/LIB.RS".to_owned(),
                kind: BuildInputKind::RustSource,
            },
        ]),
        Err(ClosureError::DuplicateOrAlias)
    );
}

#[test]
fn representative_input_mutation_changes_anchor_and_stales_prior_closure() {
    use super::plugin_product::source_closure::{BuildClosureV1, ClosureError};
    let root = ClosureTempRoot::new();
    let policy = build_policy(root.path());
    let before = BuildClosureV1::capture(root.path(), &policy).unwrap();
    std::fs::write(root.path().join("runtime.json"), b"{\"runtime\":false}\n").unwrap();
    assert_eq!(
        before.verify(root.path(), &policy),
        Err(ClosureError::DigestMismatch)
    );
    let after = BuildClosureV1::capture(root.path(), &policy).unwrap();
    assert_ne!(after.aggregate_sha256, before.aggregate_sha256);
}

#[test]
fn final_session_drift_is_rejected_after_initial_read() {
    use super::plugin_product::source_closure::{BuildClosureV1, ClosureError};
    let root = ClosureTempRoot::new();
    let policy = build_policy(root.path());
    let result = BuildClosureV1::capture_with_observer(root.path(), &policy, |index, path| {
        if index == 0 {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
            file.write_all(b"# drift\n").unwrap();
        }
    });
    assert_eq!(result, Err(ClosureError::FinalSessionDrift));
}

#[cfg(unix)]
#[test]
fn build_closure_rejects_symlink_hardlink_and_special_inputs() {
    use super::plugin_product::source_closure::{
        BuildClosurePolicy, BuildClosureV1, BuildInputKind, ClosureError, RequiredBuildInput,
    };
    use std::os::unix::fs::symlink;
    let root = ClosureTempRoot::new();
    symlink("lib.rs", root.path().join("src/alias.rs")).unwrap();
    let alias_policy = BuildClosurePolicy::new(vec![RequiredBuildInput {
        path: "src/alias.rs".to_owned(),
        kind: BuildInputKind::RustSource,
    }])
    .unwrap();
    assert_eq!(
        BuildClosureV1::capture(root.path(), &alias_policy),
        Err(ClosureError::SpecialFile)
    );

    std::fs::hard_link(
        root.path().join("src/lib.rs"),
        root.path().join("src/hard.rs"),
    )
    .unwrap();
    let hard_policy = BuildClosurePolicy::new(vec![RequiredBuildInput {
        path: "src/lib.rs".to_owned(),
        kind: BuildInputKind::RustSource,
    }])
    .unwrap();
    assert_eq!(
        BuildClosureV1::capture(root.path(), &hard_policy),
        Err(ClosureError::SpecialFile)
    );

    let fifo = root.path().join("special.fifo");
    let path = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let fifo_policy = BuildClosurePolicy::new(vec![RequiredBuildInput {
        path: "special.fifo".to_owned(),
        kind: BuildInputKind::DynamicInput,
    }])
    .unwrap();
    assert_eq!(
        BuildClosureV1::capture(root.path(), &fifo_policy),
        Err(ClosureError::SpecialFile)
    );
}

#[test]
fn dep_info_and_policy_reject_outside_root_and_parent_paths() {
    use super::plugin_product::source_closure::{
        BuildClosurePolicy, BuildInputKind, ClosureError, RequiredBuildInput,
    };
    let root = ClosureTempRoot::new();
    let outside = std::env::temp_dir().join(format!("hul-outside-{}.rs", std::process::id()));
    std::fs::write(&outside, b"outside\n").unwrap();
    std::fs::write(
        root.path().join("target/outside.d"),
        format!("target/probe: {}\n", outside.display()),
    )
    .unwrap();
    assert_eq!(
        BuildClosurePolicy::from_dep_info(
            root.path(),
            &["target/outside.d".to_owned()],
            Vec::new(),
        ),
        Err(ClosureError::OutsideRoot)
    );
    let _ = std::fs::remove_file(outside);
    assert_eq!(
        BuildClosurePolicy::new(vec![RequiredBuildInput {
            path: "../escape".to_owned(),
            kind: BuildInputKind::DynamicInput,
        }]),
        Err(ClosureError::InvalidPath)
    );
}

#[test]
fn live_plugin_product_build_closure_is_exact_and_byte_identical_twice() {
    use super::plugin_product::source_closure::{BuildClosureV1, plugin_product_build_policy};
    let policy = plugin_product_build_policy().unwrap();
    let first = BuildClosureV1::capture(&root(), &policy).unwrap();
    let second = BuildClosureV1::capture(&root(), &policy).unwrap();
    if std::env::var_os("HUL_PRINT_BUILD_CLOSURE").is_some() {
        println!("{}", serde_json::to_string(&first).unwrap());
    }
    assert_eq!(first, second);
    assert_eq!(first.rows.len(), policy.required_inputs.len());
    assert_eq!(first.rows.len(), 71);
    first.verify(&root(), &policy).unwrap();
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildClosureSummary {
    schema_version: String,
    aggregate_sha256: String,
    row_count: usize,
    final_session_revalidated: bool,
    policy: String,
    root_extension_required: bool,
}

#[test]
fn superseded_lifecycle_worker_result_is_typed_but_stale_after_authorization_correction() {
    use super::plugin_product::product_fitness::{
        ClaimCeiling, DimensionDisposition, OverallDisposition, ProductFitnessDisposition,
    };
    use super::plugin_product::source_closure::{BuildClosureV1, plugin_product_build_policy};

    let envelope: LifecycleRootEnvelope =
        serde_json::from_str(&read(LIFECYCLE_ENVELOPE_PATH)).unwrap();
    let result =
        WorkerResultV1::parse_json(&std::fs::read(root().join(LIFECYCLE_RESULT_PATH)).unwrap())
            .unwrap();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .unwrap();
    assert!(
        ArtifactWorkspace::new(root())
            .unwrap()
            .verify(&result, &envelope.lease, &envelope.work_package)
            .is_err(),
        "superseded lifecycle result must not verify corrected source bytes"
    );
    assert_eq!(
        result.base_state["root_envelope_no_claim_statement"],
        envelope.no_claim_statement
    );

    let fitness: ProductFitnessDisposition =
        serde_json::from_value(result.final_state["product_fitness_disposition"].clone()).unwrap();
    assert_eq!(fitness.overall, OverallDisposition::Blocked);
    assert!(
        fitness
            .dimensions
            .iter()
            .all(|dimension| dimension.disposition == DimensionDisposition::Blocked)
    );
    assert!(
        fitness
            .truth_layer_ceilings
            .values()
            .all(|ceiling| *ceiling == ClaimCeiling::Withheld)
    );

    let stale_summary: BuildClosureSummary =
        serde_json::from_value(result.final_state["build_closure"].clone()).unwrap();
    let current =
        BuildClosureV1::capture(&root(), &plugin_product_build_policy().unwrap()).unwrap();
    assert_ne!(stale_summary.aggregate_sha256, current.aggregate_sha256);
    assert_eq!(stale_summary.schema_version, current.schema_version);
    assert_eq!(stale_summary.row_count, current.rows.len());
    assert!(stale_summary.final_session_revalidated);
    assert_eq!(stale_summary.policy, "plugin_product_build_policy");
    assert!(stale_summary.root_extension_required);
}

#[test]
fn authorization_correction_worker_result_is_typed_but_stale_after_recovery_activation() {
    let envelope_bytes = std::fs::read(root().join(LIFECYCLE_AUTH_ENVELOPE_PATH)).unwrap();
    assert_eq!(
        format!("sha256:{}", digest(&envelope_bytes)),
        LIFECYCLE_AUTH_ENVELOPE_SHA256
    );
    let envelope: LifecycleRootEnvelope = serde_json::from_slice(&envelope_bytes).unwrap();
    envelope.work_package.validate().unwrap();
    assert_eq!(
        envelope.lease.owner.as_str(),
        "/root/plugin_plan_authorization_engineer"
    );
    assert_eq!(envelope.no_claim_statement, WORKER_NO_CLAIM);

    let result = WorkerResultV1::parse_json(
        &std::fs::read(root().join(LIFECYCLE_AUTH_RESULT_PATH)).unwrap(),
    )
    .unwrap();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .unwrap();
    assert!(
        ArtifactWorkspace::new(root())
            .unwrap()
            .verify(&result, &envelope.lease, &envelope.work_package)
            .is_err(),
        "authorization correction must stale when recovery source and tests change"
    );
    assert_eq!(result.no_claim_statement, WORKER_NO_CLAIM);
    assert_eq!(
        result.base_state["work_envelope_sha256"],
        LIFECYCLE_AUTH_ENVELOPE_SHA256
    );
    assert_eq!(
        result.final_state["status"],
        "candidate_for_root_acceptance"
    );
}

#[test]
fn recovery_activation_worker_result_is_typed_but_stale_after_read_failure_guard() {
    let envelope_bytes = std::fs::read(root().join(LIFECYCLE_RECOVERY_ENVELOPE_PATH)).unwrap();
    assert_eq!(
        format!("sha256:{}", digest(&envelope_bytes)),
        LIFECYCLE_RECOVERY_ENVELOPE_SHA256
    );
    let envelope: LifecycleRootEnvelope = serde_json::from_slice(&envelope_bytes).unwrap();
    envelope.work_package.validate().unwrap();
    assert_eq!(
        envelope.lease.owner.as_str(),
        "/root/plugin_plan_authorization_engineer"
    );
    assert_eq!(envelope.no_claim_statement, WORKER_NO_CLAIM);

    let result = WorkerResultV1::parse_json(
        &std::fs::read(root().join(LIFECYCLE_RECOVERY_RESULT_PATH)).unwrap(),
    )
    .unwrap();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .unwrap();
    assert!(
        ArtifactWorkspace::new(root())
            .unwrap()
            .verify(&result, &envelope.lease, &envelope.work_package)
            .is_err(),
        "recovery activation must stale when read-failure source and tests change"
    );
    assert_eq!(result.no_claim_statement, WORKER_NO_CLAIM);
    assert_eq!(
        result.base_state["work_envelope_sha256"],
        LIFECYCLE_RECOVERY_ENVELOPE_SHA256
    );
    assert_eq!(
        result.final_state["status"],
        "candidate_for_root_acceptance"
    );
}

#[test]
fn read_failure_guard_worker_result_is_lease_bound_and_verifies_every_artifact() {
    let envelope_bytes = std::fs::read(root().join(LIFECYCLE_READ_FAILURE_ENVELOPE_PATH)).unwrap();
    assert_eq!(
        format!("sha256:{}", digest(&envelope_bytes)),
        LIFECYCLE_READ_FAILURE_ENVELOPE_SHA256
    );
    let envelope: LifecycleRootEnvelope = serde_json::from_slice(&envelope_bytes).unwrap();
    envelope.work_package.validate().unwrap();
    assert_eq!(
        envelope.lease.owner.as_str(),
        "/root/plugin_plan_authorization_engineer"
    );
    assert_eq!(envelope.no_claim_statement, WORKER_NO_CLAIM);

    let result = WorkerResultV1::parse_json(
        &std::fs::read(root().join(LIFECYCLE_READ_FAILURE_RESULT_PATH)).unwrap(),
    )
    .unwrap();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .unwrap();
    let verified = ArtifactWorkspace::new(root())
        .unwrap()
        .verify(&result, &envelope.lease, &envelope.work_package)
        .unwrap();
    assert_eq!(verified.artifact_count(), 4);
    assert_eq!(result.no_claim_statement, WORKER_NO_CLAIM);
    assert_eq!(
        result.base_state["work_envelope_sha256"],
        LIFECYCLE_READ_FAILURE_ENVELOPE_SHA256
    );
    assert_eq!(
        result.final_state["status"],
        "candidate_for_root_acceptance"
    );
    assert_eq!(result.final_state["read_effect_failure_guard"], true);
    assert_eq!(
        result.final_state["read_only_failure_authority_closed"],
        true
    );
    assert_eq!(result.final_state["restore_calls_after_failure"], 0);
    assert_eq!(result.final_state["observe_calls_after_failure"], 0);
    assert_eq!(result.final_state["recursive_zero_write"], true);

    let artifact_paths = result
        .artifacts
        .iter()
        .map(|artifact| artifact.path.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        artifact_paths,
        BTreeSet::from([
            "validator/src/plugin_product/lifecycle/execution.rs",
            "validator/src/plugin_product/lifecycle/model.rs",
            "validator/tests/plugin_product_contract/dependency_closure.rs",
            "validator/tests/plugin_product_contract/lifecycle_contract.rs",
        ])
    );
}
