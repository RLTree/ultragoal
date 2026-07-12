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
fn emitted_worker_result_is_exact_typed_and_artifact_verified() {
    let bytes = std::fs::read(root().join(RESULT_PATH)).expect("emitted WorkerResult-v1");
    let result = WorkerResultV1::parse_json(&bytes).expect("exact WorkerResult-v1 parse");
    let envelope = envelope();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .expect("lease-bound WorkerResult-v1");
    let verified = ArtifactWorkspace::new(root())
        .unwrap()
        .verify(&result, &envelope.lease, &envelope.work_package)
        .expect("live artifact verification");
    assert_eq!(verified.artifact_count(), result.artifacts.len());
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
    let closure = closure();
    let full = aggregate(&closure, &envelope, None, &BTreeMap::new());
    let worker = aggregate(
        &closure,
        &envelope,
        Some(Authority::WorkerOwned),
        &BTreeMap::new(),
    );
    assert_eq!(result.final_state["root_extension_set_sha256"], full);
    assert_eq!(result.final_state["worker_artifact_set_sha256"], worker);
}
