fn corrective_lease() -> (WorkPackage, LeaseSpec, ScopePolicy) {
    let owned_paths = canonical_paths(&[
        R3_RESULT_PATH,
        "validator/src/distribution/cache.rs",
        "validator/src/distribution/host_capability.rs",
        "validator/src/distribution/host_effect/executor/tests.rs",
        "validator/src/distribution/host_effect/lifecycle/binding.rs",
        "validator/src/distribution/host_effect/lifecycle/tests.rs",
        "validator/src/distribution/identity.rs",
        "validator/src/distribution/marketplace.rs",
        "validator/src/distribution/package/archive.rs",
        "validator/src/distribution/package/snapshot.rs",
        "validator/src/distribution/registry_observation.rs",
        "validator/src/distribution/runtime_probe.rs",
        "validator/src/plugin_product/host_lifecycle/session.rs",
        "validator/tests/distribution_contract/isolated_journey.rs",
        "validator/tests/distribution_contract/journey_adversarial.rs",
        "validator/tests/distribution_contract/observation_races.rs",
        "validator/tests/distribution_contract/runtime_session.rs",
        "validator/tests/plugin_host_lifecycle_contract/negative.rs",
        "validator/tests/supported_package_identity_contract.rs",
    ]);
    let semantic_symbols = BTreeSet::from([
        "distribution::supported-package-cache-identity".into(),
        "distribution::marketplace-bound-journey-identity".into(),
        "plugin_product::supported-host-marketplace-join".into(),
    ]);
    let effects = BTreeSet::from([
        EffectGrant::new(
            EffectClass::WorkspaceWrite,
            "supported-package-identity-source",
        )
        .unwrap(),
        EffectGrant::new(EffectClass::Process, "offline-validation").unwrap(),
    ]);
    let owned_scope = OwnedScope {
        paths: owned_paths.clone(),
        semantic_symbols: semantic_symbols.clone(),
        generated_outputs: BTreeSet::new(),
        fixtures: BTreeSet::new(),
        effects: effects.clone(),
    };
    let package = WorkPackage {
        node_id: "supported-package-identity".into(),
        dependencies: BTreeSet::from([
            "accepted-distribution-package-install-kernel".into(),
            "accepted-supported-host-plugin-lifecycle-coordinator".into(),
        ]),
        required_tools: BTreeSet::from(["cargo-nextest".into(), "rustfmt".into(), "shasum".into()]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::new(),
        owned_scope: owned_scope.clone(),
        prerequisites: BTreeSet::from([
            "preserve-prior-package-identity-candidate".into(),
            "root-retains-manifest-version-and-claim-authority".into(),
        ]),
        outputs: BTreeSet::from([
            "marketplace-bound-distribution-journey".into(),
            "typed-worker-result-receipt".into(),
        ]),
        acceptance: BTreeSet::from([
            "same-tree-different-version-rejected".into(),
            "wrong-cache-root-rejected".into(),
            "wrong-marketplace-rejected".into(),
            "worker-result-exact-artifact-set-verified".into(),
        ]),
        claim_effect: "none".into(),
    };
    let lease = LeaseSpec {
        lease_id: "SUPPORTED-PACKAGE-IDENTITY-074-R3".into(),
        run_id: "ultragoal-successor-live-20260713-package-marketplace-identity-r3-root".into(),
        node_id: package.node_id.clone(),
        principal: Principal::Worker,
        owner: Actor::parse("/root").unwrap(),
        binding: Binding::new(R3_CONTEXT, R3_CANDIDATE).unwrap(),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::new(),
        owned_scope,
        prerequisite_evidence: PrerequisiteEvidence::default(),
        issued_tick: 1,
        heartbeat_deadline_tick: 1_000_000,
        max_retries: 2,
    };
    let policy = ScopePolicy {
        allowed_read_paths: package.read_paths.clone(),
        allowed_paths: owned_paths,
        allowed_semantic_prefixes: semantic_symbols,
        allowed_effects: effects,
        ..ScopePolicy::default()
    };
    (package, lease, policy)
}

fn canonical_paths(values: &[&str]) -> BTreeSet<CanonicalPath> {
    values
        .iter()
        .map(|value| CanonicalPath::parse(value).unwrap())
        .collect()
}

fn snapshot_tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, current: &Path, rows: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|row| row.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by_key(|row| row.file_name());
        for entry in entries {
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(root, &path, rows);
            } else {
                rows.push((
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows
}

fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}
