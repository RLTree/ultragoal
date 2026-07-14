#[test]
fn reviewer_is_disjoint_falsification_only_and_cannot_raise_claims() {
    let root = TempRoot::new();
    let mut same_actor = disposition(root.path(), |_| DimensionDisposition::Pass);
    same_actor.reviewer_actor_id = same_actor.producer_actor_id.clone();
    assert_eq!(
        same_actor.validate(root.path()),
        Err(ProductFitnessError::ActorNotDisjoint)
    );
    let mut authority = disposition(root.path(), |_| DimensionDisposition::Pass);
    authority.may_raise_claim_ceiling = true;
    assert_eq!(
        authority.validate(root.path()),
        Err(ProductFitnessError::ReviewerAuthorityInvalid)
    );
}

#[test]
fn every_host_truth_layer_is_separate_and_cannot_skip_an_unproven_predecessor() {
    let root = TempRoot::new();
    let mut disposition = disposition(root.path(), |_| DimensionDisposition::Blocked);
    disposition.truth_layer_ceilings = source_candidate_ceilings();
    disposition.validate(root.path()).unwrap();
    assert_eq!(disposition.truth_layer_ceilings.len(), 10);
    assert_eq!(
        disposition.truth_layer_ceilings[&TruthLayer::PluginsUi],
        ClaimCeiling::Withheld
    );

    disposition
        .truth_layer_ceilings
        .insert(TruthLayer::Runtime, ClaimCeiling::LiveSameSurfaceProven);
    assert_eq!(
        disposition.validate(root.path()),
        Err(ProductFitnessError::TruthLayerEscalation)
    );
}

#[cfg(unix)]
#[test]
fn symlink_evidence_is_rejected_as_a_special_file() {
    use std::os::unix::fs::symlink;
    let root = TempRoot::new();
    let mut disposition = disposition(root.path(), |_| DimensionDisposition::Pass);
    symlink("evidence.json", root.path().join("alias.json")).unwrap();
    for dimension in &mut disposition.dimensions {
        dimension.evidence.path = "alias.json".to_owned();
    }
    assert_eq!(
        disposition.validate(root.path()),
        Err(ProductFitnessError::EvidenceSpecialFile)
    );
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FitnessCases {
    schema_version: String,
    required_dimensions: Vec<FitnessDimension>,
    truth_layers: Vec<TruthLayer>,
    unsupported_ceiling: ClaimCeiling,
    substitution_rejections: Vec<String>,
}

#[test]
fn product_fitness_fixture_names_exact_dimensions_layers_and_substitutions() {
    let fixture: FitnessCases = serde_json::from_str(&super::read(
        "fixtures/plugin-product/product-fitness-cases.json",
    ))
    .unwrap();
    assert_eq!(fixture.schema_version, "HarnessProductFitnessCases-v1");
    assert_eq!(fixture.required_dimensions.len(), 5);
    assert_eq!(fixture.truth_layers, TRUTH_LAYERS);
    assert_eq!(fixture.unsupported_ceiling, ClaimCeiling::Withheld);
    assert!(
        fixture
            .substitution_rejections
            .contains(&"receipt_only".to_owned())
    );
}

#[test]
fn typed_disposition_rejects_unknown_fields() {
    let root = TempRoot::new();
    let value =
        serde_json::to_value(disposition(root.path(), |_| DimensionDisposition::Pass)).unwrap();
    let mut value = value.as_object().unwrap().clone();
    value.insert("unknown".to_owned(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<ProductFitnessDisposition>(value.into()).is_err());

    let mut wrong_schema = disposition(root.path(), |_| DimensionDisposition::Pass);
    wrong_schema.schema_version = "future".to_owned();
    assert_eq!(
        wrong_schema.validate(root.path()),
        Err(ProductFitnessError::InvalidDisposition)
    );
}
