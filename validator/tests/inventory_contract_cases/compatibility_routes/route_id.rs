const ROUTE_ID: &str = "skill-ultragoal-to-harness-ultragoal";
const LEGACY_ID: &str = "LEGACY-SKILL:ultragoal";

fn build_catalog(repo: &TestRepo) -> crate::inventory::AuthorityCatalog {
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context).build().unwrap()
}

fn route_mut(repo: &TestRepo, change: impl FnOnce(&mut Value)) {
    let path = repo.root.join("migration/authority-routes.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let route = value["routes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|route| route["route_id"] == ROUTE_ID)
        .unwrap();
    change(route);
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

fn retention_transition(legacy: &str) -> Value {
    json!({
            "compatibility_behavior": "exact-route-only",
            "compatibility_boundary": "explicit-only",
            "replacement_state": "candidate-required",
            "active_reader_writer_state": "active",
            "observed_authority_state": "compatibility-route-retained",
            "equivalence_proof": "missing",
            "physical_cleanup_state": "preserve",
            "proof_refs": [
                format!("skills/{legacy}/SKILL.md"),
                format!("skills/{legacy}/agents/openai.yaml")
            ]
    })
}

fn request_retention(repo: &TestRepo) {
    route_mut(repo, |route| {
        route["transition"] = retention_transition("ultragoal");
    });
}

fn request_all_retentions(repo: &TestRepo) {
    let path = repo.root.join("migration/authority-routes.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for route in value["routes"].as_array_mut().unwrap() {
        let Some(legacy) = route["match"]["stable_id"]
            .as_str()
            .and_then(|id| id.strip_prefix("LEGACY-SKILL:"))
        else {
            continue;
        };
        route["transition"] = retention_transition(legacy);
    }
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

fn wrapper_skill() -> String {
    "---\nname: ultragoal\ndescription: Deprecated compatibility alias for explicit `$harness-ultragoal:ultragoal` requests. Preserve the request and route it to `$harness-ultragoal:harness-ultragoal`; do not use this alias as independent workflow authority.\n---\n\n# Deprecated Compatibility Route\n\n> Compatibility warning: this legacy alias is not an independent workflow or authority. Its canonical target is `$harness-ultragoal:harness-ultragoal`.\n\n1. Preserve the user's full request, context, constraints, and authorized effects unchanged.\n2. Before routing, inspect the request and supplied context for Harness Ultragoal skill tokens. If it contains another distinct explicit Harness Ultragoal skill token or selects multiple compatibility routes, report a causal compatibility-route conflict and perform no routing or effect.\n3. Invoke `$harness-ultragoal:harness-ultragoal` with that preserved input.\n4. Follow only the canonical target's current contract. Do not restore or apply legacy lane, gate, receipt, finalizer, command, tool, helper, schema, state-store, or generated authority from this wrapper.\n5. Perform no hidden writes or external effects while resolving the route. Any later effect must remain authorized by the original request and the canonical target.\n6. Fail closed if the canonical target is unavailable: report the exact blocker and do not fall back, infer semantic equivalence, or claim adoption, discovery, runtime behavior, retirement, readiness, release, or completion.\n7. Never substitute documentation, tests, receipts, generated rows, telemetry, signatures, or provenance for the requested product behavior.\n".to_owned()
}

fn wrapper_metadata() -> String {
    "interface:\n  display_name: \"Ultragoal (Legacy Alias)\"\n  short_description: \"Route this legacy alias to its canonical skill\"\n  default_prompt: \"Use $harness-ultragoal:ultragoal to preserve and route my unchanged request through its canonical skill.\"\npolicy:\n  allow_implicit_invocation: false\n".to_owned()
}

fn write_wrapper(repo: &TestRepo, skill: &str, metadata: &str) {
    repo.write("skills/ultragoal/SKILL.md", skill.as_bytes());
    repo.write("skills/ultragoal/agents/openai.yaml", metadata.as_bytes());
}

fn has_finding(catalog: &crate::inventory::AuthorityCatalog, code: &str) -> bool {
    catalog
        .findings()
        .iter()
        .any(|finding| finding.code == code && finding.entry_id.as_deref() == Some(LEGACY_ID))
}

#[test]
fn all_exact_explicit_only_wrappers_are_retained_without_claiming_retirement() {
    let repo = TestRepo::new("compatibility-retained");
    for canonical in [
        "harness-ultragoal",
        "repository-fit",
        "routine-work",
        "diagnose-and-observe",
        "goal-run",
        "prove",
        "improve-and-maintain",
        "product-journey-review",
    ] {
        repo.skill(canonical, canonical);
    }
    for legacy in [
        "ultragoal",
        "harness-engineering",
        "agent-first-repo-init",
        "agent-first-repo-retrofit",
        "fit-repo",
        "execplan-lane",
        "agent-runtime-legibility",
        "agent-observability-stack",
        "orchestrator-reconciler",
        "proof-gate",
        "product-cohesion-gate",
        "product-fitness-gate",
        "agent-improvement-loop",
        "standards-gardener",
    ] {
        for relative in ["SKILL.md", "agents/openai.yaml"] {
            let path = format!("skills/{legacy}/{relative}");
            repo.write(&path, &fs::read(live_root().join(&path)).unwrap());
        }
    }
    request_all_retentions(&repo);
    repo.commit();

    let catalog = build_catalog(&repo);
    let entries = catalog
        .entries()
        .iter()
        .filter(|entry| entry.kind == "compatibility-route-retained")
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 14);
    assert!(entries.iter().all(|entry| {
        entry.authority_state == AuthorityState::Legacy
            && entry.active_status == ActiveStatus::Active
    }));
    assert_eq!(
        catalog
            .findings()
            .iter()
            .filter(|finding| finding.code == "compatibility_route_retained")
            .count(),
        14
    );
    let closure = catalog.closure_status();
    assert_eq!(
        closure
            .open_obligations_by_code()
            .get("compatibility_route_retained"),
        Some(&14)
    );
    assert!(
        !closure
            .blockers_by_code()
            .contains_key("compatibility_route_retained")
    );
    assert!(entries.iter().all(|entry| {
        !catalog.findings().iter().any(|finding| {
            finding.entry_id.as_deref() == Some(entry.stable_id.as_str())
                && matches!(
                    finding.code.as_str(),
                    "parallel_authority" | "invalid_compatibility_route_transition"
                )
        })
    }));
}
