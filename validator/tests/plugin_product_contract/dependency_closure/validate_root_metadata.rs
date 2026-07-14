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
