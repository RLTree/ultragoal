#[test]
fn package_surface_identity_rejects_a_different_publication_transaction() {
    let first = Repo::new("package-surface-first");
    let first_context = first.context();
    let first_catalog = catalog(&first_context);
    let first_artifact = capture_product_package(&first_context, &first_catalog).unwrap();
    let first_output = OutputRoot::new("package-surface-first");
    let mut first_tree = first_output.tree();
    let first_transaction = first_artifact
        .publish(
            &first_context,
            &first_catalog,
            &first_output.journey(&first_artifact),
            &ExpectedTree::Absent,
            &mut first_tree,
        )
        .unwrap();

    let second = Repo::new("package-surface-second");
    let second_context = second.context();
    let second_catalog = catalog(&second_context);
    let second_artifact = capture_product_package(&second_context, &second_catalog).unwrap();
    let host = HostCapabilityDeclaration::isolated(
        &first_output.root,
        &first_output.root,
        "macos-repository-output-v1",
        None,
    )
    .unwrap();
    let journey = JourneyBinding::new(
        second_artifact.snapshot().identity().clone(),
        &host,
        "local-harness-plugins",
    )
    .unwrap();

    assert_eq!(
        SurfaceIdentity::from_published_package(
            second_artifact.snapshot(),
            &first_transaction,
            &journey,
        )
        .unwrap_err()
        .id(),
        crate::distribution::DistributionErrorId::ProvenanceMismatch
    );
}

#[test]
fn generic_internal_issuance_cannot_bypass_package_or_app_registry_authority() {
    let repo = Repo::new("restricted-surface-issuance");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).unwrap();
    for surface in [IdentitySurface::Package, IdentitySurface::AppRegistry] {
        assert_eq!(
            SurfaceIdentity::new(
                artifact.snapshot().identity().clone(),
                surface,
                artifact.snapshot().inventory_sha256().into(),
                None,
            )
            .unwrap_err()
            .id(),
            crate::distribution::DistributionErrorId::ProvenanceMismatch
        );
    }
}
