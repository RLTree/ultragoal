use crate::orchestration::{CanonicalPath, EffectClass, LeaseSpec, ScopePolicy, WorkPackage};
use serde_json::Value;
use std::collections::BTreeSet;

const ENVELOPE: &[u8] = include_bytes!(
    "../../../docs/ultragoal-successor-live/work-packages/CANONICAL-PLUGIN-DEPENDENCY-CLOSURE-018.json"
);

#[test]
fn canonical_plugin_envelope_is_typed_disjoint_and_root_protected() {
    let value: Value = serde_json::from_slice(ENVELOPE).unwrap();
    let package: WorkPackage = serde_json::from_value(value["work_package"].clone()).unwrap();
    let lease: LeaseSpec = serde_json::from_value(value["lease"].clone()).unwrap();
    package.validate().unwrap();

    let policy = ScopePolicy {
        allowed_read_paths: package.read_paths.clone(),
        allowed_paths: package.owned_scope.paths.clone(),
        allowed_semantic_prefixes: BTreeSet::from(["plugin_product".to_owned()]),
        allowed_generated_outputs: package.owned_scope.generated_outputs.clone(),
        allowed_fixtures: package.owned_scope.fixtures.clone(),
        allowed_effects: package.owned_scope.effects.clone(),
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
    lease.validate(&policy).unwrap();

    assert_eq!(lease.node_id, package.node_id);
    assert_eq!(lease.safety_class, package.safety_class);
    assert!(lease.read_paths.is_subset(&package.read_paths));
    assert!(lease.owned_scope.is_subset_of(&package.owned_scope));
    assert_eq!(
        lease
            .prerequisite_evidence
            .dependency_nodes
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        package.dependencies
    );
    assert!(
        lease
            .read_paths
            .iter()
            .all(|path| { !matches!(path.as_str().split('/').next(), Some(".codex" | ".agents")) })
    );
    assert_eq!(value["protected_root_inputs"].as_array().unwrap().len(), 7);
}
