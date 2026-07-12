use super::repository_fit::{
    FitErrorId, FitMode, FitPlan, Ownership, apply, inspect, inspect_with_managed_proofs, plan,
};
use super::support::{MemoryRepo, authorization, desired, file, managed_proof};

fn absent_plan(paths: &[&str]) -> (super::repository_fit::FitPlan, MemoryRepo) {
    let desired = desired(
        paths
            .iter()
            .map(|path| file(path, path.as_bytes(), Ownership::UserOwned, &[]))
            .collect(),
    );
    let mut repo = MemoryRepo::default();
    let plan = plan(
        &inspect(FitMode::Fresh, &desired, &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    (plan, repo)
}

fn apply_error(plan: &FitPlan, repo: &mut MemoryRepo) -> FitErrorId {
    apply(plan, &authorization(plan), repo).unwrap_err().id()
}

#[test]
fn first_write_then_error_restores_absence_and_prior_bytes() {
    let (first_plan, mut repo) = absent_plan(&["A.md"]);
    repo.error_after_cas = Some(1);
    assert_eq!(
        apply(&first_plan, &authorization(&first_plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::EffectFailed
    );
    assert!(!repo.files.contains_key("A.md"));
    assert_eq!((repo.writes, repo.cas_calls), (2, 2));

    let prior = b"generated-prior";
    let desired = desired(vec![file(
        "B.md",
        b"generated-next",
        Ownership::HarnessGenerated,
        &[prior],
    )]);
    let mut repo = MemoryRepo::with("B.md", prior);
    let proof = managed_proof("B.md", prior);
    let plan = plan(
        &inspect_with_managed_proofs(FitMode::Retrofit, &desired, &[proof], &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    repo.error_after_cas = Some(1);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::EffectFailed
    );
    assert_eq!(repo.files["B.md"], prior.as_slice());
    assert_eq!((repo.writes, repo.cas_calls), (2, 2));
}

#[test]
fn later_write_then_error_restores_current_and_every_prior_mutation() {
    let (plan, mut repo) = absent_plan(&["A.md", "B.md"]);
    repo.error_after_cas = Some(2);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::EffectFailed
    );
    assert!(repo.files.is_empty());
    assert_eq!((repo.writes, repo.cas_calls), (4, 4));
}

#[test]
fn pre_write_error_restores_prior_mutations_only() {
    let (plan, mut repo) = absent_plan(&["A.md", "B.md"]);
    repo.fail_cas = Some(2);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::EffectFailed
    );
    assert!(repo.files.is_empty());
    assert_eq!((repo.writes, repo.cas_calls), (2, 3));
}

#[test]
fn foreign_post_error_bytes_are_preserved_while_safe_prior_restores() {
    let (plan, mut repo) = absent_plan(&["A.md", "B.md"]);
    repo.error_after_cas = Some(2);
    repo.corrupt_cas = Some(2);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::RollbackFailed
    );
    assert!(!repo.files.contains_key("A.md"));
    assert_eq!(repo.files["B.md"], b"corrupt-after-cas");
    assert_eq!((repo.writes, repo.cas_calls), (3, 3));
}

#[test]
fn reconciliation_read_failure_preserves_current_and_restores_prior() {
    let (plan, mut repo) = absent_plan(&["A.md", "B.md"]);
    repo.error_after_cas = Some(2);
    repo.fail_read_after_cas = Some(2);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::RollbackFailed
    );
    assert!(!repo.files.contains_key("A.md"));
    assert_eq!(repo.files["B.md"], b"B.md");
    assert_eq!((repo.writes, repo.cas_calls), (3, 3));
}

#[test]
fn root_binding_change_after_effect_error_prevents_unsafe_rollback() {
    for binding_offset in [4, 5] {
        let (plan, mut repo) = absent_plan(&["A.md", "B.md"]);
        repo.error_after_cas = Some(2);
        repo.root_swap_on_binding = Some(repo.binding_calls() + binding_offset);
        assert_eq!(apply_error(&plan, &mut repo), FitErrorId::RollbackFailed);
        assert_eq!(repo.files["A.md"], b"A.md");
        assert_eq!(repo.files["B.md"], b"B.md");
        assert_eq!((repo.writes, repo.cas_calls), (2, 2));
    }
}

#[test]
fn current_rollback_cas_race_does_not_skip_safe_prior_rollback() {
    let (plan, mut repo) = absent_plan(&["A.md", "B.md"]);
    repo.error_after_cas = Some(2);
    repo.mutate_cas = Some(3);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::RollbackFailed
    );
    assert!(!repo.files.contains_key("A.md"));
    assert_eq!(repo.files["B.md"], b"raced");
    assert_eq!((repo.writes, repo.cas_calls), (3, 4));
}

#[test]
fn current_rollback_effect_error_does_not_skip_safe_prior_rollback() {
    let (plan, mut repo) = absent_plan(&["A.md", "B.md"]);
    repo.error_after_cas = Some(2);
    repo.fail_cas = Some(3);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::RollbackFailed
    );
    assert!(!repo.files.contains_key("A.md"));
    assert_eq!(repo.files["B.md"], b"B.md");
    assert_eq!((repo.writes, repo.cas_calls), (3, 4));
}

#[test]
fn post_rollback_corruption_is_not_reported_as_an_ordinary_effect_failure() {
    let (plan, mut repo) = absent_plan(&["A.md"]);
    repo.error_after_cas = Some(1);
    repo.corrupt_cas = Some(2);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::RollbackFailed
    );
    assert_eq!(repo.files["A.md"], b"corrupt-after-cas");
    assert_eq!((repo.writes, repo.cas_calls), (2, 2));
}

#[test]
fn prior_rollback_conflict_does_not_skip_older_safe_mutation() {
    let (plan, mut repo) = absent_plan(&["A.md", "B.md", "C.md"]);
    let reads_before = repo.reads;
    repo.error_after_cas = Some(3);
    repo.mutate_cas = Some(5);
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::RollbackFailed
    );
    assert!(!repo.files.contains_key("A.md"));
    assert_eq!(repo.files["B.md"], b"raced");
    assert!(!repo.files.contains_key("C.md"));
    assert_eq!((repo.writes, repo.cas_calls), (5, 6));
    assert_eq!(repo.reads, reads_before + 9);
}

#[test]
fn prior_state_after_error_returns_original_failure_without_echoing_content() {
    let prior = b"private-managed-prior";
    let desired = desired(vec![
        file("A.md", b"private-new-file", Ownership::UserOwned, &[]),
        file(
            "B.md",
            b"private-managed-next",
            Ownership::HarnessGenerated,
            &[prior],
        ),
    ]);
    let mut repo = MemoryRepo::with("B.md", prior);
    let proof = managed_proof("B.md", prior);
    let plan = plan(
        &inspect_with_managed_proofs(FitMode::Retrofit, &desired, &[proof], &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    repo.fail_cas = Some(2);
    let failure = apply(&plan, &authorization(&plan), &mut repo).unwrap_err();
    assert_eq!(failure.id(), FitErrorId::EffectFailed);
    assert!(!repo.files.contains_key("A.md"));
    assert_eq!(repo.files["B.md"], prior.as_slice());
    let rendered = format!("{failure:?} {failure}");
    for secret in [
        "private-managed-prior",
        "private-managed-next",
        "private-new-file",
    ] {
        assert!(!rendered.contains(secret));
    }
}

#[test]
fn final_sweep_rejects_raced_current_after_later_rollback() {
    for raced in [b"B.md".as_slice(), b"foreign-after-restore".as_slice()] {
        let (plan, mut repo) = absent_plan(&["A.md", "B.md"]);
        repo.error_after_cas = Some(2);
        repo.external_on_cas = Some((4, "B.md".into(), Some(raced.to_vec())));
        assert_eq!(apply_error(&plan, &mut repo), FitErrorId::RollbackFailed);
        assert!(!repo.files.contains_key("A.md"));
        assert_eq!(repo.files["B.md"], raced);
        assert_eq!((repo.writes, repo.cas_calls), (4, 4));
    }
}

#[test]
fn final_sweep_rejects_unreadable_or_root_changed_state() {
    for root_offset in [None, Some(12), Some(13)] {
        let (plan, mut repo) = absent_plan(&["A.md", "B.md"]);
        repo.error_after_cas = Some(2);
        if let Some(offset) = root_offset {
            repo.root_swap_on_binding = Some(repo.binding_calls() + offset);
        } else {
            repo.fail_read = Some(repo.reads + 6);
        }
        assert_eq!(
            apply(&plan, &authorization(&plan), &mut repo)
                .unwrap_err()
                .id(),
            FitErrorId::RollbackFailed
        );
        assert!(repo.files.is_empty());
        assert_eq!((repo.writes, repo.cas_calls), (4, 4));
    }
}

#[test]
fn final_sweep_rejects_prior_file_removed_after_restore() {
    let prior = b"managed-prior";
    let wanted = desired(vec![
        file("A.md", b"A.md", Ownership::UserOwned, &[]),
        file(
            "B.md",
            b"managed-next",
            Ownership::HarnessGenerated,
            &[prior],
        ),
    ]);
    let mut repo = MemoryRepo::with("B.md", prior);
    let proof = managed_proof("B.md", prior);
    let plan = plan(
        &inspect_with_managed_proofs(FitMode::Retrofit, &wanted, &[proof], &mut repo).unwrap(),
        &wanted,
    )
    .unwrap();
    repo.error_after_cas = Some(2);
    repo.external_on_cas = Some((4, "B.md".into(), None));
    assert_eq!(apply_error(&plan, &mut repo), FitErrorId::RollbackFailed);
    assert!(repo.files.is_empty());
    assert_eq!((repo.writes, repo.cas_calls), (4, 4));
}
