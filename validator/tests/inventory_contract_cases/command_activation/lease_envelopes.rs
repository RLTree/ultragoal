#[test]
fn derived_lease_envelopes_refuse_unknown_or_stale_classification() {
    let valid = source_repo("lease-derived-envelope-valid");
    mutate_registry(&valid, activate_n08_lease);
    let context = LiveContext::build(inventory_request(&valid.root)).unwrap();
    InventoryBuilder::new(&context)
        .build()
        .expect("derived lease is valid");

    for (label, mutate, expected) in [
        (
            "unknown",
            Box::new(|record: &mut serde_json::Value| {
                record["changed_set"]["classification"] = "unknown".into()
            }) as Box<dyn Fn(&mut serde_json::Value)>,
            "lease envelope classification is unknown",
        ),
        (
            "changed",
            Box::new(|record: &mut serde_json::Value| {
                record["changed_set"]["categories"]["files"] =
                    serde_json::json!(["validator/src/evaluation/"])
            }) as Box<dyn Fn(&mut serde_json::Value)>,
            "lease envelope category digest is stale",
        ),
        (
            "intersection",
            Box::new(|record: &mut serde_json::Value| {
                record["changed_set"]["intersection"] =
                    serde_json::json!({"status":"intersects","categories":["files"]})
            }) as Box<dyn Fn(&mut serde_json::Value)>,
            "lease intersection disposition is stale",
        ),
        (
            "consumed-intersection",
            Box::new(|record: &mut serde_json::Value| {
                record["consumed_set"]["intersection"]["status"] = "unknown".into()
            }) as Box<dyn Fn(&mut serde_json::Value)>,
            "lease intersection disposition is unknown",
        ),
    ] {
        let repo = source_repo(&format!("lease-derived-envelope-{label}"));
        mutate_registry(&repo, |registry| {
            activate_n08_lease(registry);
            mutate(&mut registry["lease_state"]["active_records"][0]);
        });
        assert_inventory_error(&repo, expected);
    }
}

fn activate_n08_lease(registry: &mut serde_json::Value) {
    registry["pre_adoption_source"]["frontier"] = "N08_READY_N09_INTEGRATED_SOURCE_FRONTIER".into();
    registry["pre_adoption_source"]["eligible_scheduler_nodes"] = serde_json::json!(["N08"]);

    let lanes = registry["lanes"].as_array_mut().unwrap();
    let n08 = lanes.iter_mut().find(|lane| lane["id"] == "N08").unwrap();
    n08["state"] = "ready".into();
    n08["current_identity"] = serde_json::Value::Null;
    n08["ceiling"] = "adopted_reobservation_required".into();
    let lane = n08.clone();

    let scope = registry["scope_mappings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|scope| scope["scope_id"] == "WS-PLUGIN")
        .unwrap()
        .clone();
    let dependency_identities = lane["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| {
            registry["lanes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|candidate| candidate["id"] == *id)
                .unwrap()["current_identity"]
                .clone()
        })
        .collect::<Vec<_>>();

    let mut record = registry["lease_state"]["record_template"].clone();
    record["lease_id"] = "LEASE-TEST-N08".into();
    record["lane_id"] = "N08".into();
    record["owner"] = lane["owner"].clone();
    record["scope_ids"] = lane["scope_ids"].clone();
    record["branch"] = "codex/test-n08".into();
    record["worktree"] = "/tmp/test-n08".into();
    record["owned_files"] = scope["owned_roots"].clone();
    record["owned_symbols"] = scope["owned_symbols"].clone();
    record["generated_outputs"] = scope["generated_roots"].clone();
    record["fixtures"] = scope["fixture_roots"].clone();
    record["effects"] = scope["effects"].clone();
    record["consumed_set"]["dependency_identities"] =
        serde_json::Value::Array(dependency_identities);
    populate_envelopes(registry, &lane, &mut record);
    record["invalidated_by"] = lane["invalidation_contract"]["invalidated_by"].clone();
    record["status"] = "issued".into();
    record["clean_handoff"] = "pending".into();
    record["reachable_tip"] = true.into();

    registry["lease_state"]["status"] = "active".into();
    registry["lease_state"]["active_records"] = serde_json::json!([record]);
    for gate in registry["prelaunch_gates"].as_array_mut().unwrap() {
        if gate["status"] == "current" {
            let id = gate["id"].as_str().unwrap().to_owned();
            let commit = gate["observed_source_base"]["commit"]
                .as_str()
                .unwrap()
                .to_owned();
            let tree = gate["observed_source_base"]["tree"]
                .as_str()
                .unwrap()
                .to_owned();
            gate["operation_id"] = format!("prelaunch-{id}-{commit}-{tree}").into();
        }
    }
}

fn populate_envelopes(
    registry: &serde_json::Value,
    lane: &serde_json::Value,
    record: &mut serde_json::Value,
) {
    let base = registry["prelaunch_gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|gate| gate["status"] == "current")
        .unwrap()["observed_source_base"]
        .clone();
    let empty = serde_json::json!({"files":[],"symbols":[],"generated_outputs":[],"fixtures":[],"effects":[]});
    record["changed_set"] = envelope("changed", "git-diff-name-status", &base, &empty, None);
    let mut files = registry["root_freeze"]["permitted_root_paths"]
        .as_array()
        .unwrap()
        .clone();
    let mut symbols = Vec::new();
    let mut generated = Vec::new();
    let mut fixtures = Vec::new();
    let mut effects = Vec::new();
    for dependency in lane["dependencies"].as_array().unwrap() {
        let lane = registry["lanes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == *dependency)
            .unwrap();
        for id in lane["scope_ids"].as_array().unwrap() {
            let scope = registry["scope_mappings"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["scope_id"] == *id)
                .unwrap();
            files.extend(scope["contract_roots"].as_array().unwrap().clone());
            files.extend(scope["owned_roots"].as_array().unwrap().clone());
            symbols.extend(scope["owned_symbols"].as_array().unwrap().clone());
            generated.extend(scope["generated_roots"].as_array().unwrap().clone());
            fixtures.extend(scope["fixture_roots"].as_array().unwrap().clone());
            effects.extend(scope["effects"].as_array().unwrap().clone());
        }
    }
    let categories = serde_json::json!({"files":sorted(files),"symbols":sorted(symbols),"generated_outputs":sorted(generated),"fixtures":sorted(fixtures),"effects":sorted(effects)});
    record["consumed_set"] = envelope(
        "consumed",
        "registry-scope-consumption",
        &base,
        &categories,
        Some(record["consumed_set"]["dependency_identities"].clone()),
    );
}

fn envelope(
    kind: &str,
    algorithm: &str,
    base: &serde_json::Value,
    categories: &serde_json::Value,
    dependencies: Option<serde_json::Value>,
) -> serde_json::Value {
    let base = serde_json::json!({"commit":base["commit"],"tree":base["tree"]});
    let digests = serde_json::json!({"files":digest(&categories["files"]),"symbols":digest(&categories["symbols"]),"generated_outputs":digest(&categories["generated_outputs"]),"fixtures":digest(&categories["fixtures"]),"effects":digest(&categories["effects"])});
    let aggregate = digest(
        &serde_json::json!({"kind":kind,"algorithm":algorithm,"version":"v1","from":base,"to":base,"categories":categories,"category_digests":digests}),
    );
    let mut value = serde_json::json!({"kind":kind,"algorithm":algorithm,"version":"v1","from":base,"to":base,"classification":"classified","categories":categories,"category_digests":digests,"aggregate_digest":aggregate,"intersection":{"status":"none","categories":[]}});
    if let Some(dependencies) = dependencies {
        value["dependency_identities"] = dependencies;
    }
    value
}

fn sorted(mut values: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    values.sort_by(|left, right| left.as_str().cmp(&right.as_str()));
    values.dedup();
    values
}
fn digest(value: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(value.to_string().as_bytes()))
}

fn mutate_registry(repo: &TestRepo, mutate: impl FnOnce(&mut serde_json::Value)) {
    let path = repo.root.join("LANE_REGISTRY.json");
    let mut registry = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    mutate(&mut registry);
    fs::write(path, serde_json::to_vec(&registry).unwrap()).unwrap();
}
