fn catalog(repo: &TestRepo) -> AuthorityCatalog {
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context).build().unwrap()
}

fn has_code(catalog: &AuthorityCatalog, code: &str) -> bool {
    catalog
        .findings()
        .iter()
        .any(|finding| finding.code == code)
}

#[test]
fn supported_manifest_is_never_compared_to_the_unsupported_legacy_draft() {
    let repo = TestRepo::new("manifest-authority-separation");
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"harness-ultragoal","version":"0.0.0-test"}"#,
    );
    repo.write(
        "plugin-manifest-draft.json",
        br#"{"legacy":true,"unsupported_shape":"intentionally-different"}"#,
    );
    repo.commit();

    let catalog = catalog(&repo);
    assert!(!has_code(&catalog, "projection_drift"));
    assert!(catalog.findings().iter().any(|finding| {
        finding.code == "parallel_authority"
            && finding.relative_path.as_deref() == Some("plugin-manifest-draft.json")
    }));
    assert!(has_code(
        &catalog,
        "projection_requires_canonical_reconciliation"
    ));
}

#[test]
fn deterministic_repeat_and_byte_exact_projection() {
    let repo = TestRepo::new("repeat");
    repo.skill("harness-ultragoal", "harness-ultragoal");
    repo.commit();
    let first = catalog(&repo);
    let second = catalog(&repo);
    assert_eq!(first.catalog_id(), second.catalog_id());
    assert_eq!(
        first.to_canonical_json().unwrap(),
        second.to_canonical_json().unwrap()
    );
    let projection = first.to_canonical_json().unwrap();
    assert!(first.compare_projection(&projection).unwrap().matches);
}

#[test]
fn omitted_duplicate_and_renamed_components_are_findings() {
    let repo = TestRepo::new("component-errors");
    repo.skill("renamed-front-door", "harness-ultragoal");
    repo.skill("duplicate-front-door", "harness-ultragoal");
    repo.commit();
    let catalog = catalog(&repo);
    assert!(has_code(&catalog, "missing_required_component"));
    assert!(has_code(&catalog, "duplicate_stable_id"));
    assert!(has_code(&catalog, "renamed_required_component"));
}

#[test]
fn stale_or_tampered_projection_is_rejected() {
    let repo = TestRepo::new("projection");
    repo.commit();
    let catalog = catalog(&repo);
    let mut value: serde_json::Value =
        serde_json::from_slice(&catalog.to_canonical_json().unwrap()).unwrap();
    value["catalog_id"] = serde_json::json!("sha256:tampered");
    let comparison = catalog
        .compare_projection(&serde_json::to_vec(&value).unwrap())
        .unwrap();
    assert!(!comparison.matches);
    assert_eq!(comparison.findings[0].code, "stale_or_tampered_projection");
    let canonical_value: serde_json::Value =
        serde_json::from_slice(&catalog.to_canonical_json().unwrap()).unwrap();
    let reformatted = serde_json::to_vec_pretty(&canonical_value).unwrap();
    assert!(!catalog.compare_projection(&reformatted).unwrap().matches);
    let duplicate_key = br#"{"catalog_id":"first","catalog_id":"second"}"#;
    assert!(!catalog.compare_projection(duplicate_key).unwrap().matches);
}

#[cfg(unix)]
#[test]
fn symlink_escape_is_a_causal_finding() {
    let repo = TestRepo::new("symlink");
    let outside = repo.root.with_file_name(format!(
        "{}-outside",
        repo.root.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("SKILL.md"), b"---\nname: escape\n---\n").unwrap();
    let skill = repo.root.join("skills/escape");
    fs::create_dir_all(&skill).unwrap();
    std::os::unix::fs::symlink(outside.join("SKILL.md"), skill.join("SKILL.md")).unwrap();
    repo.commit();
    let catalog = catalog(&repo);
    assert!(has_code(&catalog, "symlink_path_escape"));
    assert!(
        !catalog
            .entries()
            .iter()
            .any(|entry| entry.relative_path == "skills/escape/SKILL.md")
    );
    let _ = fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[test]
fn escaped_inventory_symlinks_are_not_dereferenced() {
    let repo = TestRepo::new("symlink-no-dereference");
    let outside = repo.root.with_file_name(format!(
        "{}-outside",
        repo.root.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir_all(&outside).unwrap();
    let outside_json = outside.join("private.json");
    fs::write(
        &outside_json,
        br#"{"_meta":{"generator":"OUTSIDE-GENERATOR","inputs":["OUTSIDE-INPUT"]},"$ref":"OUTSIDE-REF"}"#,
    )
    .unwrap();
    fs::remove_file(repo.root.join(".codex-plugin/plugin.json")).unwrap();
    std::os::unix::fs::symlink(&outside_json, repo.root.join(".codex-plugin/plugin.json")).unwrap();
    fs::create_dir_all(repo.root.join("schemas")).unwrap();
    std::os::unix::fs::symlink(&outside_json, repo.root.join("schemas/escaped.schema.json"))
        .unwrap();
    fs::create_dir_all(repo.root.join("generated")).unwrap();
    std::os::unix::fs::symlink(&outside_json, repo.root.join("generated/escaped.json")).unwrap();
    repo.commit();
    let catalog = catalog(&repo);
    assert!(has_code(&catalog, "symlink_path_escape"));
    let json = String::from_utf8(catalog.to_canonical_json().unwrap()).unwrap();
    for escaped in [
        ".codex-plugin/plugin.json",
        "schemas/escaped.schema.json",
        "generated/escaped.json",
    ] {
        assert!(
            !catalog
                .entries()
                .iter()
                .any(|entry| entry.relative_path == escaped)
        );
    }
    for secret in ["OUTSIDE-GENERATOR", "OUTSIDE-INPUT", "OUTSIDE-REF"] {
        assert!(!json.contains(secret));
    }
    let _ = fs::remove_dir_all(outside);
}

#[test]
fn declared_name_and_reference_canaries_are_not_emitted() {
    let repo = TestRepo::new("metadata-canary");
    repo.skill("safe-skill", "SECRET_CANARY");
    repo.write("schemas/canary.schema.json", br#"{"$ref":"SECRET_CANARY"}"#);
    repo.write(
        "schemas/fragment-canary.schema.json",
        br#"{"$ref":"safe.schema.json#/$defs/SECRET_CANARY"}"#,
    );
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"harness-ultragoal","$ref":"SECRET_CANARY"}"#,
    );
    repo.commit();
    let catalog = catalog(&repo);
    let json = String::from_utf8(catalog.to_canonical_json().unwrap()).unwrap();
    assert!(!json.contains("SECRET_CANARY"));
    assert!(has_code(&catalog, "invalid_component_metadata"));
    assert!(catalog.entries().iter().any(|entry| {
        entry.stable_id == "INVALID-SKILL:safe-skill"
            && entry.active_status == crate::inventory::ActiveStatus::ContextOnly
    }));
}
