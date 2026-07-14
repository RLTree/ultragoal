#[cfg(unix)]
#[test]
fn projection_capture_rejects_restore_substitution_and_special_files() {
    use std::os::unix::fs::symlink;

    let repo = source_repo("projection-session");
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    let bytes = catalog.to_canonical_json().unwrap();
    let projection = repo.root.join("target/catalog.json");
    fs::create_dir_all(projection.parent().unwrap()).unwrap();
    fs::write(&projection, &bytes).unwrap();

    let supplied = catalog.compare_projection(&bytes).unwrap();
    assert!(supplied.matches);
    assert!(!supplied.input_verified_in_session);
    assert!(!supplied.authority_eligible);

    let capture = catalog
        .capture_projection_file(&context, "target/catalog.json")
        .unwrap();
    let displaced = repo.root.join("target/displaced.json");
    fs::rename(&projection, &displaced).unwrap();
    fs::write(&projection, &bytes).unwrap();
    let error = capture.finish().unwrap_err();
    assert!(error.to_string().contains("stale"));
    fs::remove_file(displaced).unwrap();

    let verified = catalog
        .compare_projection_file(&context, "target/catalog.json")
        .unwrap();
    assert!(verified.matches);
    assert!(verified.input_verified_in_session);
    assert!(!verified.authority_eligible);

    let symlink_path = repo.root.join("target/projection-link.json");
    symlink("catalog.json", &symlink_path).unwrap();
    assert!(
        catalog
            .compare_projection_file(&context, "target/projection-link.json")
            .unwrap_err()
            .to_string()
            .ends_with("unsafe-input")
    );

    let hardlink_path = repo.root.join("target/projection-hard.json");
    fs::hard_link(&projection, &hardlink_path).unwrap();
    assert!(
        catalog
            .compare_projection_file(&context, "target/projection-hard.json")
            .is_err()
    );
    fs::remove_file(hardlink_path).unwrap();

    let fifo_path = repo.root.join("target/projection-fifo.json");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo_path)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        catalog
            .compare_projection_file(&context, "target/projection-fifo.json")
            .unwrap_err()
            .to_string()
            .ends_with("unsafe-input")
    );
}

#[derive(Deserialize)]
struct ClosureCases {
    schema_version: String,
    cases: Vec<ClosureCase>,
}

#[derive(Deserialize)]
struct ClosureCase {
    case_id: String,
    expected_disposition: String,
    causal_code: Option<String>,
}

#[test]
fn closure_fixture_names_every_guard_class_without_duplicates() {
    let path = live_root().join("fixtures/inventory-authority/closure-cases.json");
    let fixture: ClosureCases = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(fixture.schema_version, "InventoryClosureCases-v1");
    let ids = fixture
        .cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), fixture.cases.len());
    assert!(fixture.cases.iter().all(|case| {
        matches!(
            case.expected_disposition.as_str(),
            "active" | "candidate" | "abort" | "non-authoritative"
        ) && (case.causal_code.is_some() || case.case_id == "active-compiled-api")
    }));
}
