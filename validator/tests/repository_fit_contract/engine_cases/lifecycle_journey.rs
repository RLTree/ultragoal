use super::*;

#[test]
pub(crate) fn fresh_inspect_plan_apply_verify_reapply_and_rollback() {
    let desired = desired(vec![file(
        ".codex/AGENTS.md",
        b"canonical\n",
        Ownership::HarnessGenerated,
        &[],
    )]);
    let mut repo = MemoryRepo::default();
    let inspection = inspect(FitMode::Fresh, &desired, &mut repo).unwrap();
    assert_eq!(inspection.classification(), &RepositoryClass::Fresh);
    let initial_plan = plan(&inspection, &desired).unwrap();
    assert_eq!(initial_plan.mutations().len(), 1);
    let transaction = apply(&initial_plan, &authorization(&initial_plan), &mut repo).unwrap();
    assert_eq!(verify(&desired, &mut repo).unwrap().matched_files(), 1);
    let second = plan(
        &inspect(FitMode::Retrofit, &desired, &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    assert!(second.mutations().is_empty());
    assert!(verify(&desired, &mut repo).unwrap().idempotent());
    rollback(transaction, &mut repo).unwrap();
    assert!(!repo.files.contains_key(".codex/AGENTS.md"));
}

#[test]
pub(crate) fn partial_retrofit_updates_only_known_generated_bytes() {
    let prior = b"generated-v1\n";
    let desired = desired(vec![
        file(
            ".codex/generated.md",
            b"generated-v2\n",
            Ownership::HarnessGenerated,
            &[prior],
        ),
        file("AGENTS.md", b"user-template\n", Ownership::UserOwned, &[]),
    ]);
    let mut repo = MemoryRepo::with(".codex/generated.md", prior);
    let proof = managed_proof(".codex/generated.md", prior);
    let inspection = super::super::repository_fit::inspect_with_managed_proofs(
        FitMode::Retrofit,
        &desired,
        &[proof],
        &mut repo,
    )
    .unwrap();
    assert_eq!(inspection.classification(), &RepositoryClass::Partial);
    let plan = plan(&inspection, &desired).unwrap();
    assert_eq!(plan.mutations().len(), 2);
    let managed_check = plan
        .checks()
        .iter()
        .find(|check| check.path().as_str() == ".codex/generated.md")
        .unwrap();
    assert!(matches!(
        managed_check.provenance(),
        super::super::repository_fit::OwnershipProvenance::AdoptedManifest { .. }
    ));
    assert!(managed_check.prior_proof_sha256().is_some());
    apply(&plan, &authorization(&plan), &mut repo).unwrap();
    assert_eq!(repo.files[".codex/generated.md"], b"generated-v2\n");
    assert_eq!(repo.files["AGENTS.md"], b"user-template\n");
}

#[test]
pub(crate) fn known_generated_digest_without_opaque_state_proof_remains_a_conflict() {
    let prior = b"generated-v1\n";
    let desired = desired(vec![file(
        ".codex/generated.md",
        b"generated-v2\n",
        Ownership::HarnessGenerated,
        &[prior],
    )]);
    let mut repo = MemoryRepo::with(".codex/generated.md", prior);
    let plan = plan(
        &inspect(FitMode::Retrofit, &desired, &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    assert_eq!(plan.conflicts().len(), 1);
    assert!(plan.mutations().is_empty());
}

#[test]
pub(crate) fn stale_or_unknown_managed_proof_is_rejected_not_silently_ignored() {
    let prior = b"generated-v1\n";
    let wanted = file(
        ".codex/generated.md",
        b"generated-v2\n",
        Ownership::HarnessGenerated,
        &[prior],
    );
    let stale = super::super::repository_fit::ownership::issue_managed_prior_proof(
        sha(b'a'),
        sha(b'b'),
        sha(b'd'),
        super::super::repository_fit::CanonicalPath::parse(".codex/generated.md").unwrap(),
        super::super::repository_fit::digest(prior),
        wanted.provenance().clone(),
    )
    .unwrap();
    let desired = desired(vec![wanted]);
    let mut repo = MemoryRepo::with(".codex/generated.md", prior);
    assert_eq!(
        super::super::repository_fit::inspect_with_managed_proofs(
            FitMode::Retrofit,
            &desired,
            &[stale],
            &mut repo,
        )
        .unwrap_err()
        .id(),
        FitErrorId::InvalidSpec
    );
    assert_eq!(repo.writes, 0);
}

#[test]
pub(crate) fn unknown_user_or_generated_edits_are_explicit_conflicts_and_never_write() {
    for ownership in [Ownership::UserOwned, Ownership::HarnessGenerated] {
        let desired = desired(vec![file("AGENTS.md", b"wanted", ownership, &[])]);
        let mut repo = MemoryRepo::with("AGENTS.md", b"local-user-change");
        let plan = plan(
            &inspect(FitMode::Retrofit, &desired, &mut repo).unwrap(),
            &desired,
        )
        .unwrap();
        assert_eq!(plan.conflicts().len(), 1);
        assert_eq!(
            apply(&plan, &authorization(&plan), &mut repo)
                .unwrap_err()
                .id(),
            FitErrorId::Conflict
        );
        assert_eq!(repo.files["AGENTS.md"], b"local-user-change");
        assert_eq!(repo.writes, 0);
    }
}

#[test]
pub(crate) fn authorization_candidate_plan_and_root_are_exactly_bound() {
    let desired = desired(vec![file("A.md", b"wanted", Ownership::UserOwned, &[])]);
    let mut repo = MemoryRepo::default();
    let plan = plan(
        &inspect(FitMode::Fresh, &desired, &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    let wrong =
        super::super::repository_fit::PlanAuthorization::new(sha(b'a'), sha(b'b'), sha(b'e'))
            .unwrap();
    assert_eq!(
        apply(&plan, &wrong, &mut repo).unwrap_err().id(),
        FitErrorId::Unauthorized
    );
    repo.root_swap_on_binding = Some(repo.reads + 1);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::StaleBinding
    );
    assert_eq!(repo.writes, 0);
}

#[test]
pub(crate) fn compare_exchange_race_is_rejected_without_overwrite() {
    let desired = desired(vec![file("A.md", b"wanted", Ownership::UserOwned, &[])]);
    let mut repo = MemoryRepo::default();
    let plan = plan(
        &inspect(FitMode::Fresh, &desired, &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    repo.mutate_cas = Some(1);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::Conflict
    );
    assert_eq!(repo.files["A.md"], b"raced");
    assert_eq!(repo.writes, 0);
}

#[test]
pub(crate) fn later_effect_failure_rolls_back_every_prior_mutation() {
    let desired = desired(vec![
        file("A.md", b"a", Ownership::UserOwned, &[]),
        file("B.md", b"b", Ownership::UserOwned, &[]),
    ]);
    let mut repo = MemoryRepo::default();
    let plan = plan(
        &inspect(FitMode::Fresh, &desired, &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    repo.fail_cas = Some(2);
    assert!(apply(&plan, &authorization(&plan), &mut repo).is_err());
    assert!(repo.files.is_empty());
    assert_eq!(repo.writes, 2);
}

#[test]
pub(crate) fn corrupted_effect_is_not_a_false_pass_and_failed_rollback_is_explicit() {
    let desired = desired(vec![file("A.md", b"a", Ownership::UserOwned, &[])]);
    let mut repo = MemoryRepo::default();
    let plan = plan(
        &inspect(FitMode::Fresh, &desired, &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    repo.corrupt_cas = Some(1);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::RollbackFailed
    );
}

#[test]
pub(crate) fn inspect_plan_and_verify_are_zero_write_and_deterministic() {
    let desired = desired(vec![file("A.md", b"a", Ownership::UserOwned, &[])]);
    let mut repo = MemoryRepo::with("A.md", b"a");
    let first = inspect(FitMode::Retrofit, &desired, &mut repo).unwrap();
    let first_plan = plan(&first, &desired).unwrap();
    let second = inspect(FitMode::Retrofit, &desired, &mut repo).unwrap();
    let second_plan = plan(&second, &desired).unwrap();
    verify(&desired, &mut repo).unwrap();
    assert_eq!(first.inspection_sha256(), second.inspection_sha256());
    assert_eq!(first_plan.plan_sha256(), second_plan.plan_sha256());
    assert_eq!(repo.writes, 0);
}
