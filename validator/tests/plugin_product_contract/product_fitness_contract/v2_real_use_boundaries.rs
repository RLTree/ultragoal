#[test]
fn typed_disposition_rejects_legacy_dogfood_schema() {
    let root = TempRoot::new();
    let mut legacy = v2_disposition(root.path());
    legacy.schema_version = "harness-ultragoal.dogfood-receipt.v1".to_owned();
    assert_eq!(
        legacy.validate(root.path()),
        Err(ProductFitnessError::InvalidDisposition)
    );
}

#[test]
fn v2_rejects_duplicate_surface_evidence_and_unbound_observations() {
    let root = TempRoot::new();
    let mut duplicate = v2_disposition(root.path());
    let source = duplicate
        .surface_identities
        .as_ref()
        .unwrap()
        .source
        .evidence
        .clone();
    duplicate
        .surface_identities
        .as_mut()
        .unwrap()
        .package
        .evidence = source;
    assert_eq!(
        duplicate.validate(root.path()),
        Err(ProductFitnessError::WrongSurface)
    );

    let mut stale = v2_disposition(root.path());
    stale
        .public_entry_observation
        .as_mut()
        .unwrap()
        .evidence
        .current_session = false;
    assert_eq!(
        stale.validate(root.path()),
        Err(ProductFitnessError::EvidenceStale)
    );

    let mut unbound = v2_disposition(root.path());
    unbound
        .real_work_observation
        .as_mut()
        .unwrap()
        .repository_identity = digest(b"other repository");
    assert_eq!(
        unbound.validate(root.path()),
        Err(ProductFitnessError::InvalidRepository)
    );
}

#[test]
fn v2_requires_legacy_dogfood_rejection_and_permits_marketplace_from_installed() {
    let root = TempRoot::new();
    let mut omitted = v2_disposition(root.path());
    omitted
        .substitution_rejections
        .remove("legacy_dogfood_receipt");
    assert_eq!(
        omitted.validate(root.path()),
        Err(ProductFitnessError::MissingSubstitutionRejection)
    );

    let mut marketplace = v2_disposition(root.path());
    marketplace.evidence_class = Some(EvidenceClass::Installed);
    marketplace.operator_kind = Some(OperatorKind::Agent);
    marketplace.dimensions.iter_mut().for_each(|item| {
        if item.dimension == FitnessDimension::Continuance {
            item.disposition = DimensionDisposition::Blocked;
        }
    });
    marketplace.overall = OverallDisposition::Blocked;
    marketplace.surface_identities.as_mut().unwrap().marketplace =
        observed_surface(root.path(), "marketplace", "marketplace-identity");
    marketplace.claimed_surface = Some(TruthLayer::Marketplace);
    for layer in [
        TruthLayer::Source,
        TruthLayer::Package,
        TruthLayer::Marketplace,
    ] {
        marketplace
            .truth_layer_ceilings
            .insert(layer, ClaimCeiling::LiveSameSurfaceProven);
    }
    marketplace.validate(root.path()).unwrap();
}
