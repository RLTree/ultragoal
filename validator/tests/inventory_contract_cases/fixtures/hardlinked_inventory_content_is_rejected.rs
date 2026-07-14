#[cfg(unix)]
#[test]
fn hardlinked_inventory_content_is_rejected() {
    let repo = TestRepo::new("hardlink");
    let outside = repo.root.with_file_name(format!(
        "{}-outside",
        repo.root.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir_all(&outside).unwrap();
    let outside_json = outside.join("private.json");
    fs::write(&outside_json, br#"{"private":"content"}"#).unwrap();
    fs::create_dir_all(repo.root.join("generated")).unwrap();
    fs::hard_link(&outside_json, repo.root.join("generated/hardlink.json")).unwrap();
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("hard links"));
    let _ = fs::remove_dir_all(outside);
}

#[test]
fn overdeep_inventory_walk_fails_causally() {
    let repo = TestRepo::new("walk-depth");
    let mut path = repo.root.join("schemas");
    for _ in 0..65 {
        path.push("d");
    }
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("deep.schema.json"), b"{}\n").unwrap();
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("exceeds depth"));
}

#[test]
fn generated_and_schema_drift_are_discovered() {
    let repo = TestRepo::new("generated");
    let source = b"current source";
    repo.write("source.txt", source);
    repo.write(
        "migration/generated-surface-authority.json",
        br#"{"schema_version":"GeneratedSurfaceAuthority-v2","contract_id":"harness-ultragoal-successor-contract-v2","surfaces":[{"disposition":"canonical_projection","output":"docs/generated/current-input.json","generator":"HCT-INVENTORY","recipe":"input-digest-index-v1","inputs":["source.txt"]},{"disposition":"canonical_projection","output":"docs/generated/stale-input.json","generator":"HCT-INVENTORY","recipe":"input-digest-index-v1","inputs":["source.txt"]}]}"#,
    );
    repo.write("docs/generated/stale.json", br#"{"value":1}"#);
    repo.write(
        "docs/generated/stale-input.json",
        br#"{"_meta":{"generator":"HCT-INVENTORY","inputs":[{"path":"source.txt","sha256":"0000000000000000000000000000000000000000000000000000000000000000"}],"recipe":"input-digest-index-v1"},"entries":[]}"#,
    );
    repo.write(
        "docs/generated/canary.json",
        br#"{"_meta":{"generator":"SECRET_CANARY","inputs":[{"path":"SECRET_CANARY","sha256":"0000000000000000000000000000000000000000000000000000000000000000"}]}}"#,
    );
    let source_digest = format!("{:x}", Sha256::digest(source));
    let current = format!(
        "{{\"_meta\":{{\"generator\":\"HCT-INVENTORY\",\"inputs\":[{{\"path\":\"source.txt\",\"sha256\":\"{source_digest}\"}}]}}}}"
    );
    repo.write("docs/generated/current-input.json", current.as_bytes());
    repo.write(
        "schemas/broken.schema.json",
        br#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$ref":"missing.schema.json"}"#,
    );
    repo.commit();
    let catalog = catalog(&repo);
    assert!(has_code(&catalog, "generated_surface_missing_provenance"));
    assert!(has_code(&catalog, "generated_output_drift"));
    assert!(has_code(&catalog, "unregistered_generated_surface"));
    assert!(has_code(&catalog, "generated_output_regeneration_required"));
    assert!(has_code(&catalog, "invalid_json_reference_source"));
    let catalog_json = String::from_utf8(catalog.to_canonical_json().unwrap()).unwrap();
    assert!(!catalog_json.contains("SECRET_CANARY"));
    assert!(
        catalog
            .generated_surfaces()
            .entries()
            .iter()
            .any(|entry| entry.relative_path == "docs/generated/stale.json")
    );
}

#[test]
fn inventory_build_is_zero_write_across_worktree_and_git() {
    let repo = TestRepo::new("zero-write");
    repo.commit();
    let before = snapshot(&repo.root);
    let _catalog = catalog(&repo);
    let after = snapshot(&repo.root);
    assert_eq!(before, after);
}
