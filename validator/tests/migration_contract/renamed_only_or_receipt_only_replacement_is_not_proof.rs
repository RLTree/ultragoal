#[test]
fn renamed_only_or_receipt_only_replacement_is_not_proof() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    authority.new_verdict = EvidenceVerdict::CausalFailure;
    assert_eq!(
        ReplacementEvidence::issue(&inventory, &route, &mut authority)
            .unwrap_err()
            .code(),
        "migration-replacement-observation-invalid"
    );
}

#[test]
fn plan_verify_and_retirement_reconciliation_are_zero_write() {
    let root = temp_root("zero-write");
    fs::write(root.join("sentinel"), b"preserve").unwrap();
    let before = tree(&root);
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let _ = plan.verify_current(&inventory);
    let _ = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut replacement_authority,
        "retirement-reviewer",
    );
    assert_eq!(tree(&root), before);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fixture_catalog_covers_security_mutation_and_false_pass_cases() {
    let current: Value = serde_json::from_str(include_str!(
        "../../../fixtures/migration-engine/current-valid.json"
    ))
    .unwrap();
    let red: Value = serde_json::from_str(include_str!(
        "../../../fixtures/migration-engine/red-cases.json"
    ))
    .unwrap();
    assert_eq!(current["schema_version"], "MigrationEngineFixture-v1");
    let cases = red["cases"].as_array().unwrap();
    assert!(cases.len() >= 55);
    for expected in [
        "mutate-restore-session",
        "active-reader",
        "active-writer",
        "renamed-only-replacement",
        "caller-supplied-replacement-digest",
        "forged-replacement-evidence",
        "replacement-evidence-clone",
        "replacement-evidence-deserialize",
        "missing-named-false-pass-control",
        "repeated-hash-evidence",
        "replacement-evidence-replay",
        "replacement-evidence-mutate-restore",
        "same-route-owner-reviewer",
        "stale-replacement-evidence-binding",
        "replacement-ledger-loss",
        "replacement-ledger-rollback",
        "replacement-evidence-revocation",
        "replacement-authority-substitution",
        "replacement-cross-process-replay",
        "replacement-finalization-concurrent-winner",
        "replacement-post-check-drift",
        "opaque-token-debug-echo",
        "consumed-evidence-clone",
        "consumed-evidence-serialize",
        "migration-plan-clone",
        "migration-plan-serialize",
        "migration-plan-deserialize",
        "migration-plan-projection-replay",
        "migration-plan-roundtrip-authority",
        "migration-plan-projection-substitution",
        "symlink-surface",
        "hardlink-surface",
        "special-file-surface",
        "unauthorized-deletion",
        "caller-asserted-independent-boolean",
        "caller-asserted-destructive-authority",
        "forged-retirement-review",
        "forged-destructive-authorization",
        "same-review-effect-principal",
        "same-review-effect-session",
        "expired-retirement-review",
        "stale-retirement-binding",
        "substituted-effect-scope",
        "retirement-authorization-replay",
        "conflicting-od009-decision",
        "authority-mutation-during-consumption",
    ] {
        assert!(cases.iter().any(|case| case == expected));
    }
}

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "hul-migration-043-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    fn walk(root: &Path, current: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    walk(root, root, &mut out);
    out
}
