pub(super) fn write_registry(repo: &TestRepo, value: &Value) {
    repo.write(
        "migration/authority-routes.json",
        &serde_json::to_vec(value).unwrap(),
    );
}

fn copy_live(repo: &TestRepo, path: &str) {
    repo.write(path, &fs::read(live_root().join(path)).unwrap());
}

fn copy_reader_evidence(repo: &TestRepo) {
    let receipt = fs::read(live_root().join(READER_PROOF)).unwrap();
    let value: Value = serde_json::from_slice(&receipt).unwrap();
    for row in value["artifacts"].as_array().unwrap() {
        copy_live(repo, row["path"].as_str().unwrap());
    }
    copy_live(
        repo,
        "validator/src/inventory/agent_reader_guard_digests.rs",
    );
    for path in READER_SOURCES {
        copy_live(repo, path);
    }
    copy_live(repo, "plugin-manifest-draft.json");
    for path in AGENT_MANIFESTS {
        copy_live(repo, path);
    }
    repo.write(READER_PROOF, &receipt);
}

pub(super) fn prepare(repo: &TestRepo, cases: &[Case], reader_proof: bool) {
    for case in cases {
        copy_live(repo, case.0);
    }
    if reader_proof {
        copy_reader_evidence(repo);
    }
    let mut value = registry(repo);
    let routes = value["routes"].as_array_mut().unwrap();
    routes.retain(|row| row["route_id"] != BROAD_ROUTE);
    routes.extend(cases.iter().copied().map(route));
    write_registry(repo, &value);
}

pub(super) fn catalog_result(
    repo: &TestRepo,
) -> Result<crate::inventory::AuthorityCatalog, crate::inventory::InventoryError> {
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context).build()
}

pub(super) fn catalog(repo: &TestRepo) -> crate::inventory::AuthorityCatalog {
    catalog_result(repo).unwrap()
}

pub(super) fn entry<'a>(
    catalog: &'a crate::inventory::AuthorityCatalog,
    case: Case,
) -> &'a crate::inventory::InventoryEntry {
    let id = format!("LEGACY-AGENT:{}", case.0);
    catalog
        .entries()
        .iter()
        .find(|entry| entry.stable_id == id)
        .unwrap()
}
