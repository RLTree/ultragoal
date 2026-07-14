use super::*;

#[test]
pub(crate) fn root_change_during_inspection_fails_closed() {
    let desired = desired(vec![file("A.md", b"a", Ownership::UserOwned, &[])]);
    let mut repo = MemoryRepo::default();
    repo.root_swap_on_binding = Some(2);
    assert_eq!(
        inspect(FitMode::Fresh, &desired, &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::StaleBinding
    );
    assert_eq!(repo.writes, 0);
}

#[test]
pub(crate) fn already_matching_file_is_a_precondition_not_a_stale_plan_shortcut() {
    let desired = desired(vec![file("A.md", b"a", Ownership::UserOwned, &[])]);
    let mut repo = MemoryRepo::with("A.md", b"a");
    let plan = plan(
        &inspect(FitMode::Retrofit, &desired, &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    assert!(plan.mutations().is_empty());
    repo.files.insert("A.md".into(), b"external-edit".to_vec());
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::Conflict
    );
    assert_eq!(repo.files["A.md"], b"external-edit");
    assert_eq!(repo.writes, 0);
}

#[test]
pub(crate) fn external_change_to_matching_file_rolls_back_owned_mutations_only() {
    let desired = desired(vec![
        file("A.md", b"a", Ownership::UserOwned, &[]),
        file("B.md", b"b", Ownership::UserOwned, &[]),
    ]);
    let mut repo = MemoryRepo::with("A.md", b"a");
    let plan = plan(
        &inspect(FitMode::Retrofit, &desired, &mut repo).unwrap(),
        &desired,
    )
    .unwrap();
    repo.external_after_cas = Some(("A.md".into(), b"external-edit".to_vec()));
    assert_eq!(
        apply(&plan, &authorization(&plan), &mut repo)
            .unwrap_err()
            .id(),
        FitErrorId::VerificationFailed
    );
    assert_eq!(repo.files["A.md"], b"external-edit");
    assert!(!repo.files.contains_key("B.md"));
}
