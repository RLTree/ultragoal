#[test]
fn generated_authority_v3_keeps_registry_and_collection_bounds() {
    let surfaces = (0..257)
        .map(|index| {
            json!({
                "disposition":"canonical_projection",
                "output":format!("generated/{index:03}.json"),
                "generator":"HCT-INVENTORY",
                "recipe":"input-digest-index-v1",
                "inputs":["source.txt"]
            })
        })
        .collect();
    rejects("generated-surface-count", &registry(surfaces));

    let inputs = (0..257)
        .map(|index| format!("inputs/{index:03}.txt"))
        .collect::<Vec<_>>();
    rejects(
        "generated-input-count",
        &registry(vec![json!({
            "disposition":"canonical_projection", "output":"generated/many-inputs.json",
            "generator":"HCT-INVENTORY", "recipe":"input-digest-index-v1", "inputs":inputs
        })]),
    );

    let replacements = (0..257)
        .map(|index| format!("HCT-X{index:03}"))
        .collect::<Vec<_>>();
    rejects(
        "generated-replacement-count",
        &registry(vec![json!({
            "disposition":"retained_context", "output":"generated/many-targets.json",
            "sha256":"a".repeat(64), "reason":"context",
            "replacement_targets":replacements, "preserve":true,
            "physical_deletion_authorized":false
        })]),
    );

    let oversized = TestRepo::new("generated-registry-bytes");
    let mut bytes = registry(Vec::new());
    bytes.extend(std::iter::repeat_n(b' ', 1024 * 1024));
    oversized.write(REGISTRY, &bytes);
    oversized.commit();
    let context = LiveContext::build(inventory_request(&oversized.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("byte limit"));
}

#[test]
fn retained_context_missing_and_tamper_are_causal_findings() {
    let missing = TestRepo::new("generated-retained-missing");
    let missing_output = "generated/missing-context.json";
    missing.write(
        REGISTRY,
        &registry(vec![retained(missing_output, &"a".repeat(64))]),
    );
    missing.commit();
    let context = LiveContext::build(inventory_request(&missing.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some(missing_output)
            && finding.code == "retained_context_output_missing"
    }));
    assert!(!catalog.closure_status().is_closed());
    assert!(
        catalog
            .closure_status()
            .blockers_by_code()
            .contains_key("retained_context_output_missing")
    );

    let tampered = TestRepo::new("generated-retained-tampered");
    let tampered_output = "generated/tampered-context.json";
    tampered.write(tampered_output, b"actual bytes");
    tampered.write(
        REGISTRY,
        &registry(vec![retained(tampered_output, &"b".repeat(64))]),
    );
    tampered.commit();
    let context = LiveContext::build(inventory_request(&tampered.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some(tampered_output)
            && finding.code == "retained_context_digest_mismatch"
    }));
    let entry = catalog
        .entries()
        .iter()
        .find(|entry| entry.stable_id == format!("GENERATED:{tampered_output}"))
        .expect("tampered physical surface remains inventoried");
    assert_eq!(entry.authority_state, AuthorityState::Legacy);
    assert_eq!(entry.active_status, ActiveStatus::Active);
    assert_eq!(entry.generator, None);
    assert!(entry.references.is_empty());
    assert!(!catalog.closure_status().is_closed());
    assert!(
        catalog
            .closure_status()
            .blockers_by_code()
            .contains_key("retained_context_digest_mismatch")
    );
}

#[cfg(unix)]
#[test]
fn retained_context_symlink_and_special_outputs_are_rejected() {
    let linked = TestRepo::new("generated-retained-symlink");
    linked.write("target.bin", b"target");
    let linked_output = "generated/linked-context.json";
    fs::create_dir_all(linked.root.join("generated")).unwrap();
    std::os::unix::fs::symlink("../target.bin", linked.root.join(linked_output)).unwrap();
    linked.write(
        REGISTRY,
        &registry(vec![retained(linked_output, &sha256(b"target"))]),
    );
    linked.commit();
    let context = LiveContext::build(inventory_request(&linked.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("regular non-symlink file"));

    let special = TestRepo::new("generated-retained-special");
    let special_output = "generated/special-context.json";
    fs::create_dir_all(special.root.join("generated")).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(special.root.join(special_output))
            .status()
            .unwrap()
            .success()
    );
    special.write(
        REGISTRY,
        &registry(vec![retained(special_output, &"c".repeat(64))]),
    );
    special.commit();
    let context = LiveContext::build(inventory_request(&special.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some(special_output)
            && finding.code == "retained_context_output_not_regular"
    }));
}
