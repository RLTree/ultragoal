#[test]
fn registry_text_cannot_launder_a_non_wrapper_or_missing_target() {
    let repo = TestRepo::new("compatibility-registry-only");
    repo.skill("harness-ultragoal", "harness-ultragoal");
    repo.skill("ultragoal", "ultragoal");
    request_retention(&repo);
    repo.commit();
    let catalog = build_catalog(&repo);
    assert!(has_finding(&catalog, "invalid_compatibility_route_wrapper"));
    assert!(has_finding(
        &catalog,
        "invalid_compatibility_route_transition"
    ));
    assert!(has_finding(&catalog, "parallel_authority"));
    assert!(!has_finding(&catalog, "compatibility_route_retained"));

    let repo = TestRepo::new("compatibility-target-missing");
    write_wrapper(&repo, &wrapper_skill(), &wrapper_metadata());
    request_retention(&repo);
    repo.commit();
    let catalog = build_catalog(&repo);
    assert!(has_finding(
        &catalog,
        "invalid_compatibility_route_transition"
    ));
    assert!(has_finding(&catalog, "parallel_authority"));

    let repo = TestRepo::new("compatibility-target-ambiguous");
    repo.skill("harness-ultragoal", "harness-ultragoal");
    repo.skill("duplicate-canonical", "harness-ultragoal");
    write_wrapper(&repo, &wrapper_skill(), &wrapper_metadata());
    request_retention(&repo);
    repo.commit();
    let catalog = build_catalog(&repo);
    assert!(has_finding(
        &catalog,
        "invalid_compatibility_route_transition"
    ));
    assert!(has_finding(&catalog, "parallel_authority"));
}

#[test]
fn policy_conflict_guard_and_route_only_body_are_structural_requirements() {
    for (label, skill, metadata) in [
        (
            "missing-policy",
            wrapper_skill(),
            wrapper_metadata().replace("policy:\n  allow_implicit_invocation: false\n", ""),
        ),
        (
            "missing-conflict-guard",
            wrapper_skill().replace("another distinct ", "another "),
            wrapper_metadata(),
        ),
        (
            "extra-workflow",
            format!("{}8. Run `legacy-command --apply`.\n", wrapper_skill()),
            wrapper_metadata(),
        ),
    ] {
        let repo = TestRepo::new(label);
        repo.skill("harness-ultragoal", "harness-ultragoal");
        write_wrapper(&repo, &skill, &metadata);
        request_retention(&repo);
        repo.commit();
        let catalog = build_catalog(&repo);
        assert!(has_finding(&catalog, "invalid_compatibility_route_wrapper"));
        assert!(has_finding(&catalog, "parallel_authority"));
        assert!(!has_finding(&catalog, "compatibility_route_retained"));
    }
}

#[test]
fn wrong_target_broad_match_and_forged_proof_fail_closed() {
    let repo = TestRepo::new("compatibility-wrong-target");
    for skill in ["harness-ultragoal", "prove"] {
        repo.skill(skill, skill);
    }
    write_wrapper(&repo, &wrapper_skill(), &wrapper_metadata());
    route_mut(&repo, |route| {
        route["canonical_target"] = json!("SKILL:prove")
    });
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("exact compiled proof"));

    for (label, mutation) in [("broad-match", "broad"), ("forged-proof", "proof")] {
        let repo = TestRepo::new(label);
        request_retention(&repo);
        route_mut(&repo, |route| match mutation {
            "broad" => route["match"] = json!({"kind": "legacy-skill-route-witness"}),
            _ => route["transition"]["proof_refs"][1] = json!("evidence/forged.json"),
        });
        repo.commit();
        let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
        let error = InventoryBuilder::new(&context).build().unwrap_err();
        assert!(error.to_string().contains("exact compiled proof"));
    }
}

#[test]
fn compatibility_witness_never_proves_equivalence_or_retirement() {
    let repo = TestRepo::new("compatibility-retirement-overclaim");
    route_mut(&repo, |route| {
        route["transition"] = json!({
            "compatibility_behavior": "verified",
            "compatibility_boundary": "adopted",
            "replacement_state": "verified",
            "active_reader_writer_state": "none-verified",
            "observed_authority_state": "context-only",
            "equivalence_proof": "verified",
            "physical_cleanup_state": "preserve",
            "proof_refs": [
                "skills/ultragoal/SKILL.md",
                "skills/ultragoal/agents/openai.yaml"
            ]
        });
    });
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("exact compiled proof"));
}
