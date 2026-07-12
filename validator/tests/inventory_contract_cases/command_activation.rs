use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request, live_root, snapshot};
use serde::Deserialize;
use std::fs;

const SOURCES: &[&str] = &[
    "validator/src/api_witness.rs",
    "validator/src/lib.rs",
    "validator/src/command_witness.rs",
    "validator/src/cli/successor/catalog.rs",
    "validator/src/cli/successor/model.rs",
    "validator/src/inventory/mod.rs",
    "validator/src/inventory/builder.rs",
    "validator/src/inventory/digest.rs",
    "validator/src/inventory/fs.rs",
    "validator/src/inventory/projection.rs",
    "validator/src/inventory/registry/command_activation.rs",
    "validator/src/inventory/registry/data.rs",
    "validator/src/inventory/registry/integrity.rs",
    "validator/src/inventory/registry/mod.rs",
    "validator/src/inventory/registry/semantic.rs",
    "validator/src/inventory/registry/sources.rs",
    "validator/src/inventory/registry/topology.rs",
    "validator/src/inventory/types.rs",
    "validator/src/inventory/validate/duplicates.rs",
    "validator/src/inventory/validate.rs",
    "validator/examples/hct_inventory.rs",
    "validator/tests/public_api_witness.rs",
];

fn copy_sources(repo: &TestRepo) {
    let source_root = live_root();
    for relative in SOURCES {
        repo.write(relative, &fs::read(source_root.join(relative)).unwrap());
    }
}

fn source_repo(label: &str) -> TestRepo {
    let repo = TestRepo::new(label);
    repo.write(".gitignore", b"target/\n");
    copy_sources(&repo);
    repo.commit();
    repo
}

#[test]
fn exact_current_witness_sources_activate_apis_but_not_command_definitions() {
    let repo = source_repo("activation-current");
    let before = snapshot(&repo.root);
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    let after = snapshot(&repo.root);
    assert_eq!(
        before, after,
        "inventory read path wrote into the repository"
    );
    assert!(
        !catalog
            .findings()
            .iter()
            .any(|finding| finding.code == "activation_witness_source_set_unverified")
    );
    assert!(
        catalog
            .entries()
            .iter()
            .filter(|entry| entry.stable_id.starts_with("API:"))
            .any(|entry| entry.active_status == ActiveStatus::Active)
    );
    assert!(
        catalog
            .entries()
            .iter()
            .filter(|entry| entry.stable_id.starts_with("COMMAND:"))
            .all(|entry| entry.active_status == ActiveStatus::Candidate)
    );
    assert_eq!(
        catalog
            .source_registry_counts()
            .get("verified_command_handler_activations"),
        Some(&0)
    );
    assert_eq!(
        catalog
            .source_registry_counts()
            .get("activation_witness_sources"),
        Some(&SOURCES.len())
    );
    for entry in catalog
        .entries()
        .iter()
        .filter(|entry| entry.generator.as_deref() == Some("HCT-INVENTORY:compiled-api-witness"))
    {
        for source in SOURCES {
            assert!(
                entry.input_provenance.iter().any(|path| path == source),
                "{} omits activation dependency {source}",
                entry.stable_id
            );
        }
    }
    let closure = catalog.closure_status();
    assert!(!closure.is_closed());
    assert!(
        closure
            .blockers_by_code()
            .contains_key("candidate_component_not_active")
    );
}

#[test]
fn absent_or_stale_witness_sources_fail_closed_without_echoing_bytes() {
    let absent = TestRepo::new("activation-source-absent");
    absent.commit();
    let context = LiveContext::build(inventory_request(&absent.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(
        catalog
            .findings()
            .iter()
            .any(|finding| { finding.code == "activation_witness_source_set_unverified" })
    );
    assert!(
        catalog
            .entries()
            .iter()
            .filter(|entry| entry.stable_id.starts_with("API:"))
            .all(|entry| entry.active_status != ActiveStatus::Active)
    );

    for (label, relative) in [
        (
            "activation-source-stale-semantic",
            "validator/src/inventory/registry/semantic.rs",
        ),
        (
            "activation-source-stale-status",
            "validator/src/inventory/types.rs",
        ),
        (
            "activation-source-stale-audit",
            "validator/src/inventory/validate.rs",
        ),
        (
            "activation-source-stale-export",
            "validator/examples/hct_inventory.rs",
        ),
        (
            "activation-source-stale-external-witness",
            "validator/tests/public_api_witness.rs",
        ),
    ] {
        let stale = TestRepo::new(label);
        copy_sources(&stale);
        stale.write(relative, b"SECRET_CANARY_STALE\n");
        stale.commit();
        let context = LiveContext::build(inventory_request(&stale.root)).unwrap();
        let error = InventoryBuilder::new(&context).build().unwrap_err();
        assert!(error.to_string().ends_with("stale-input"), "{relative}");
        assert!(!error.to_string().contains("SECRET_CANARY_STALE"));
    }

    let private_export = TestRepo::new("activation-source-private-root-export");
    copy_sources(&private_export);
    let api_before = fs::read(private_export.root.join("validator/src/api_witness.rs")).unwrap();
    let lib_path = private_export.root.join("validator/src/lib.rs");
    let lib = fs::read_to_string(&lib_path).unwrap();
    let private = lib.replace("pub mod inventory;", "mod inventory;");
    assert_ne!(private, lib, "root export canary did not match lib.rs");
    fs::write(&lib_path, private).unwrap();
    assert_eq!(
        fs::read(private_export.root.join("validator/src/api_witness.rs")).unwrap(),
        api_before,
        "internal API witness changed with the root-export control"
    );
    private_export.commit();
    let context = LiveContext::build(inventory_request(&private_export.root)).unwrap();
    assert!(
        InventoryBuilder::new(&context)
            .build()
            .unwrap_err()
            .to_string()
            .ends_with("stale-input")
    );

    for (label, relative) in [
        (
            "activation-source-omitted-semantic",
            "validator/src/inventory/registry/semantic.rs",
        ),
        ("activation-source-omitted-lib", "validator/src/lib.rs"),
        (
            "activation-source-omitted-external-witness",
            "validator/tests/public_api_witness.rs",
        ),
    ] {
        let omitted = TestRepo::new(label);
        copy_sources(&omitted);
        fs::remove_file(omitted.root.join(relative)).unwrap();
        omitted.commit();
        let context = LiveContext::build(inventory_request(&omitted.root)).unwrap();
        assert!(
            InventoryBuilder::new(&context)
                .build()
                .unwrap_err()
                .to_string()
                .ends_with("missing-row"),
            "{relative}"
        );
    }
}

#[cfg(unix)]
#[test]
fn witness_source_symlink_hardlink_and_fifo_are_rejected_before_activation() {
    use std::os::unix::fs::symlink;

    fn assert_special_controls(relative: &str, link_target: &str, label: &str) {
        let linked = TestRepo::new(&format!("activation-{label}-symlink"));
        copy_sources(&linked);
        fs::remove_file(linked.root.join(relative)).unwrap();
        symlink(link_target, linked.root.join(relative)).unwrap();
        linked.commit();
        let context = LiveContext::build(inventory_request(&linked.root)).unwrap();
        assert!(
            InventoryBuilder::new(&context)
                .build()
                .unwrap_err()
                .to_string()
                .ends_with("unsafe-input")
        );

        let hard = TestRepo::new(&format!("activation-{label}-hardlink"));
        copy_sources(&hard);
        let source = hard.root.join(relative);
        let target = hard
            .root
            .join(format!("validator/{label}-hardlink-target.rs"));
        fs::remove_file(&source).unwrap();
        fs::write(&target, fs::read(live_root().join(relative)).unwrap()).unwrap();
        fs::hard_link(&target, &source).unwrap();
        hard.commit();
        let context = LiveContext::build(inventory_request(&hard.root)).unwrap();
        assert!(
            InventoryBuilder::new(&context)
                .build()
                .unwrap_err()
                .to_string()
                .ends_with("unsafe-input")
        );

        let fifo = TestRepo::new(&format!("activation-{label}-fifo"));
        copy_sources(&fifo);
        fs::remove_file(fifo.root.join(relative)).unwrap();
        fifo.commit();
        assert!(
            std::process::Command::new("mkfifo")
                .arg(fifo.root.join(relative))
                .status()
                .unwrap()
                .success()
        );
        let context = LiveContext::build(inventory_request(&fifo.root)).unwrap();
        assert!(
            InventoryBuilder::new(&context)
                .build()
                .unwrap_err()
                .to_string()
                .ends_with("unsafe-input")
        );
    }

    for (relative, target, label) in [
        (
            "validator/src/inventory/registry/semantic.rs",
            "topology.rs",
            "semantic",
        ),
        ("validator/src/lib.rs", "api_witness.rs", "lib"),
        (
            "validator/tests/public_api_witness.rs",
            "../src/api_witness.rs",
            "public-witness",
        ),
    ] {
        assert_special_controls(relative, target, label);
    }
}

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
