use super::*;

pub(crate) fn adopted_claims() -> Vec<ClaimSpec> {
    [
        ("CL-SOURCE", "source_only"),
        ("CL-PACKAGE", "package_only"),
        ("CL-INSTALL", "installed_bytes_only"),
        ("CL-DISCOVERY", "discovered_only"),
        ("CL-RUNTIME", "runtime_observed"),
        ("CL-FIT", "repository_fit"),
        ("CL-ROUTINE", "routine_verified"),
        ("CL-OBSERVABILITY", "observability_verified"),
        ("CL-STRICT", "strict_profile_verified"),
        ("CL-ORCHESTRATION", "orchestration_verified"),
        ("CL-EVAL-IMPROVEMENT", "improvement_candidate"),
        ("CL-REAL-JOURNEY", "journey_verified"),
        ("CL-RELEASE", "release_candidate"),
        ("CL-COMPLETION", "complete"),
    ]
    .into_iter()
    .map(|(claim_id, dimension)| ClaimSpec {
        claim_id: claim_id.to_owned(),
        maximum_dimensions: vec![dimension.to_owned()],
    })
    .collect()
}

pub(crate) fn policy(code: &str, dimensions: &[&str]) -> InventoryPolicy {
    InventoryPolicy {
        code: code.to_owned(),
        scope_surface: "inventory".to_owned(),
        repair: repair(
            &format!("repair-{code}"),
            AuthorityRequirement::Root,
            EffectClass::PlannedWrite,
        ),
        ceiling_reductions: reduction(dimensions),
    }
}

pub(crate) fn verify(
    catalog: &DependencyActionCatalog,
    candidate_id: Option<&str>,
) -> Result<(), StateError> {
    policy_authority::verify_bound(
        catalog,
        CONTEXT_ID,
        AUTHORITY_CATALOG_ID,
        CONTEXT_ID,
        candidate_id,
        &BTreeSet::new(),
    )
}

#[test]
pub(crate) fn exact_adopted_claim_policy_issues_and_initializes_only_its_dimensions() {
    let mut accepted = spec();
    accepted.claims = adopted_claims();
    let catalog = catalog(accepted);
    assert_ne!(catalog.catalog_id(), catalog.spec_id());
    verify(&catalog, Some(CANDIDATE_ID)).unwrap();

    let state = derive_bound(inputs(), &catalog).unwrap();
    let actual = state
        .claim_ceilings()
        .iter()
        .map(|ceiling| (ceiling.claim_id(), ceiling.dimensions()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual.len(), 14);
    for claim in adopted_claims() {
        assert_eq!(
            actual[claim.claim_id.as_str()],
            &claim.maximum_dimensions.into_iter().collect()
        );
    }
}

#[test]
pub(crate) fn adopted_claim_authority_rejects_unknown_claims_and_dimensions() {
    let accepted = spec();
    let adopted = accepted.claims.clone();
    let mut unknown_claim = accepted.clone();
    unknown_claim.claims[0].claim_id = "CL-DECOY".to_owned();
    assert_eq!(
        PolicyAuthority::from_adopted_claim_registry(
            CLAIM_REGISTRY_ID,
            adopted.clone(),
            unknown_claim,
        )
        .err(),
        Some(StateError::InvalidCatalog(
            "policy-claim-set-mismatch".to_owned()
        ))
    );
    let mut unknown_dimension = accepted;
    unknown_dimension.claims[0]
        .maximum_dimensions
        .push("forged".to_owned());
    assert_eq!(
        PolicyAuthority::from_adopted_claim_registry(
            CLAIM_REGISTRY_ID,
            adopted,
            unknown_dimension,
        )
        .err(),
        Some(StateError::InvalidCatalog(
            "policy-claim-set-mismatch".to_owned()
        ))
    );
}

#[test]
pub(crate) fn inventory_impact_coverage_rejects_missing_extra_duplicate_and_conflicting_rows() {
    assert_eq!(
        catalog_for_codes(spec(), BTreeSet::from(["required".to_owned()])),
        Err(StateError::InvalidCatalog(
            "policy-inventory-impact-missing".to_owned()
        ))
    );
    let mut extra = spec();
    extra.inventory_policies.push(policy("extra", &["runtime"]));
    assert_eq!(
        catalog_for_codes(extra, BTreeSet::new()),
        Err(StateError::InvalidCatalog(
            "policy-inventory-impact-extra".to_owned()
        ))
    );
    let mut duplicate = spec();
    duplicate.inventory_policies = vec![
        policy("duplicate", &["runtime"]),
        policy("duplicate", &["source"]),
    ];
    let duplicate_claims = duplicate.claims.clone();
    assert_eq!(
        PolicyAuthority::from_adopted_claim_registry(
            CLAIM_REGISTRY_ID,
            duplicate_claims,
            duplicate,
        )
        .err(),
        Some(StateError::InvalidCatalog(
            "policy-inventory-impact-duplicate".to_owned()
        ))
    );
    let mut conflict = spec();
    let mut conflicting = policy("conflict", &["runtime"]);
    conflicting
        .ceiling_reductions
        .extend(reduction(&["source"]));
    conflict.inventory_policies.push(conflicting);
    let conflict_claims = conflict.claims.clone();
    assert_eq!(
        PolicyAuthority::from_adopted_claim_registry(CLAIM_REGISTRY_ID, conflict_claims, conflict,)
            .err(),
        Some(StateError::InvalidCatalog(
            "policy-inventory-impact-conflict".to_owned()
        ))
    );
}

#[test]
pub(crate) fn untrusted_self_digest_and_decoy_shunting_never_initialize_ceilings() {
    let mut forged = spec();
    forged.claims = vec![
        ClaimSpec {
            claim_id: "CL-COMPLETION".to_owned(),
            maximum_dimensions: vec!["complete".to_owned()],
        },
        ClaimSpec {
            claim_id: "CL-DECOY".to_owned(),
            maximum_dimensions: vec!["blocked".to_owned()],
        },
    ];
    let untrusted = DependencyActionCatalog::from_untrusted_spec(forged).unwrap();
    assert_eq!(
        derive_bound(inputs(), &untrusted),
        Err(StateError::InvalidCatalog(
            "policy-authority-missing".to_owned()
        ))
    );
}
