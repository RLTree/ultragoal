use super::*;

#[test]
pub(crate) fn every_permit_binding_and_integrity_mutation_fails_closed() {
    let catalog = catalog(spec());
    for (field, value, expected) in [
        (
            "context",
            "sha256:stale-context",
            "policy-context-binding-mismatch",
        ),
        (
            "authority-catalog",
            "sha256:stale-authority",
            "policy-authority-catalog-binding-mismatch",
        ),
        (
            "candidate",
            "sha256:stale-candidate",
            "policy-candidate-binding-mismatch",
        ),
        (
            "permit-id",
            "sha256:self-digest",
            "policy-permit-catalog-mismatch",
        ),
    ] {
        let mut mutated = catalog.clone();
        mutated.test_mutate_permit(field, value);
        assert_eq!(
            verify(&mutated, Some(CANDIDATE_ID)),
            Err(StateError::InvalidCatalog(expected.to_owned())),
            "{field}"
        );
    }
}

#[test]
pub(crate) fn permit_substitution_and_concurrent_derivation_preserve_one_binding() {
    let mut target = catalog(spec());
    let mut donor_spec = spec();
    donor_spec.claims[0].maximum_dimensions.remove(0);
    let donor = catalog(donor_spec);
    target.test_permit_from(&donor);
    assert_eq!(
        verify(&target, Some(CANDIDATE_ID)),
        Err(StateError::InvalidCatalog(
            "policy-permit-catalog-mismatch".to_owned()
        ))
    );

    let catalog = Arc::new(catalog(spec()));
    let ids = std::thread::scope(|scope| {
        (0..16)
            .map(|_| {
                let catalog = Arc::clone(&catalog);
                scope.spawn(move || {
                    derive_bound(inputs(), &catalog)
                        .unwrap()
                        .state_id()
                        .to_owned()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<BTreeSet<_>>()
    });
    assert_eq!(ids.len(), 1);
}
