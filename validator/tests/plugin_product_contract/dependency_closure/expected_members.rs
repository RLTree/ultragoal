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
                    .unwrap_or_else(|| panic!("protected metadata member {} missing", member.path))
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
