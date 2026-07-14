use super::*;

#[test]
pub(crate) fn final_sweep_rejects_unreadable_or_root_changed_state() {
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
pub(crate) fn final_sweep_rejects_prior_file_removed_after_restore() {
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
