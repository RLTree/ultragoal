const CANDIDATE_ROOT: &str = "docs/ultragoal-contract-2026-07-successor-candidate-v1";
const REGISTRY: &str = "migration/non-authoritative-contexts.json";

fn registry() -> Value {
    json!({
        "schema_version": "NonAuthoritativeContextRegistry-v1",
        "contract_id": "harness-ultragoal-successor-contract-v2",
        "contexts": [{
            "context_id": "successor-candidate-v1",
            "root": CANDIDATE_ROOT,
            "zip_include_manifest_sha256": "2fb9b8e68c105ff92d6436801bf205d4cc528dd330292c89b0005909aab5231c",
            "content_set_digest": "f450645e73f0c324b2c9d8ca041142fff3ed4535c815ac80f87ed39c8a72d795",
            "contract_manifest_sha256": "7489750e9a42ed6c50b64ec07c30242302b501956fdf4726ce56bbf6e680d91d",
            "candidate_contract_id": "harness-ultragoal-successor-contract-candidate-v1",
            "authority": {"binding": false},
            "active_contract_replaced": false,
            "status": "candidate_for_independent_review"
        }]
    })
}

fn copy_candidate(repo: &TestRepo) {
    let source = live_root().join(CANDIDATE_ROOT);
    for entry in walkdir::WalkDir::new(&source)
        .follow_links(false)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file())
    {
        let relative = entry.path().strip_prefix(&source).unwrap();
        repo.write(
            &format!("{CANDIDATE_ROOT}/{}", relative.to_string_lossy()),
            &fs::read(entry.path()).unwrap(),
        );
    }
}

fn exact_repo(label: &str) -> TestRepo {
    let repo = TestRepo::new(label);
    copy_candidate(&repo);
    repo.write(REGISTRY, &serde_json::to_vec(&registry()).unwrap());
    repo
}

fn catalog(repo: &TestRepo) -> Result<AuthorityCatalog, InventoryError> {
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context).build()
}

fn assert_fallback(repo: &TestRepo, code: &str) {
    let catalog = catalog(repo).unwrap();
    assert!(
        catalog
            .findings()
            .iter()
            .any(|finding| finding.code == code)
    );
    let candidate_entries = catalog
        .entries()
        .iter()
        .filter(|entry| entry.relative_path.starts_with(CANDIDATE_ROOT))
        .collect::<Vec<_>>();
    assert!(!candidate_entries.is_empty());
    assert!(candidate_entries.iter().all(|entry| {
        entry.authority_state == AuthorityState::Legacy
            && entry.active_status == ActiveStatus::Active
    }));
}

#[test]
fn exact_current_candidate_bundle_is_context_only() {
    let repo = exact_repo("context-exact");
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    let context = catalog
        .entries()
        .iter()
        .filter(|entry| {
            entry.relative_path.starts_with(CANDIDATE_ROOT)
                && entry.authority_state == AuthorityState::Context
                && entry.active_status == ActiveStatus::ContextOnly
        })
        .collect::<Vec<_>>();
    assert_eq!(context.len(), 18);
    assert!(context.iter().all(|entry| {
        entry.input_provenance.contains(&REGISTRY.to_owned())
            && entry
                .input_provenance
                .contains(&format!("{CANDIDATE_ROOT}/ZIP_INCLUDE_MANIFEST.json"))
    }));
    assert!(!catalog.entries().iter().any(|entry| {
        entry.relative_path.starts_with(CANDIDATE_ROOT)
            && entry.authority_state == AuthorityState::Legacy
    }));
    let registry = catalog
        .entries()
        .iter()
        .find(|entry| entry.stable_id == "CONTEXT-SCOPE-REGISTRY")
        .unwrap();
    assert_eq!(registry.authority_state, AuthorityState::Canonical);
    assert_eq!(registry.active_status, ActiveStatus::Active);
}

#[test]
fn missing_registry_is_an_explicit_product_error() {
    let repo = TestRepo::new("context-no-registry");
    copy_candidate(&repo);
    fs::remove_file(repo.root.join(REGISTRY)).unwrap();
    repo.commit();
    let error = catalog(&repo).unwrap_err().to_string();
    assert!(error.contains(REGISTRY));
    assert!(error.contains("required"));
}

#[test]
fn malformed_spoofed_and_unknown_registry_rows_fail_closed() {
    let mut cases = Vec::new();
    for (label, pointer, replacement) in [
        ("wrong-root", "/contexts/0/root", json!("docs/spoof")),
        ("wrong-contract", "/contract_id", json!("wrong-contract")),
        (
            "wrong-candidate",
            "/contexts/0/candidate_contract_id",
            json!("wrong-candidate"),
        ),
        (
            "wrong-digest",
            "/contexts/0/zip_include_manifest_sha256",
            json!("0000000000000000000000000000000000000000000000000000000000000000"),
        ),
        ("binding", "/contexts/0/authority/binding", json!(true)),
        (
            "replaced",
            "/contexts/0/active_contract_replaced",
            json!(true),
        ),
    ] {
        let mut value = registry();
        *value.pointer_mut(pointer).unwrap() = replacement;
        cases.push((label, serde_json::to_vec(&value).unwrap()));
    }
    let valid = String::from_utf8(serde_json::to_vec(&registry()).unwrap()).unwrap();
    cases.push((
        "unknown-field",
        valid.replacen("{", "{\"unknown\":true,", 1).into_bytes(),
    ));
    cases.push((
        "duplicate-key",
        valid
            .replacen(
                "{",
                "{\"schema_version\":\"NonAuthoritativeContextRegistry-v1\",",
                1,
            )
            .into_bytes(),
    ));
    for (label, bytes) in cases {
        let repo = TestRepo::new(label);
        repo.write(
            &format!("{CANDIDATE_ROOT}/00-READ-ME-FIRST.md"),
            b"candidate",
        );
        repo.write(REGISTRY, &bytes);
        repo.commit();
        assert_fallback(&repo, "invalid_non_authoritative_context_registry");
    }
}
