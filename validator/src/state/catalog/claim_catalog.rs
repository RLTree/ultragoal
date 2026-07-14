use super::*;

pub(crate) fn canonical_claims(mut claims: Vec<ClaimSpec>) -> Result<Vec<ClaimSpec>, StateError> {
    if claims.is_empty() || claims.len() > super::super::limits::MAX_CLAIMS {
        return Err(StateError::InvalidCatalog(
            "policy-claim-set-invalid".to_owned(),
        ));
    }
    let mut ids = BTreeSet::new();
    for claim in &mut claims {
        if !super::super::limits::valid_id(&claim.claim_id)
            || claim.maximum_dimensions.is_empty()
            || !ids.insert(claim.claim_id.clone())
            || claim
                .maximum_dimensions
                .iter()
                .any(|dimension| !super::super::limits::valid_id(dimension))
        {
            return Err(StateError::InvalidCatalog(
                "policy-claim-set-invalid".to_owned(),
            ));
        }
        let unique_dimensions = claim.maximum_dimensions.iter().collect::<BTreeSet<_>>();
        if unique_dimensions.len() != claim.maximum_dimensions.len() {
            return Err(StateError::InvalidCatalog(
                "policy-claim-set-invalid".to_owned(),
            ));
        }
        claim.maximum_dimensions.sort();
    }
    claims.sort_by(|left, right| left.claim_id.cmp(&right.claim_id));
    Ok(claims)
}

pub(crate) fn validate_inventory_impacts(policies: &[InventoryPolicy]) -> Result<(), StateError> {
    let mut codes = BTreeSet::new();
    for policy in policies {
        if !codes.insert(policy.code.as_str()) {
            return Err(StateError::InvalidCatalog(
                "policy-inventory-impact-duplicate".to_owned(),
            ));
        }
        let mut claims = BTreeMap::new();
        for reduction in &policy.ceiling_reductions {
            if claims
                .insert(reduction.claim_id.as_str(), &reduction.dimensions)
                .is_some()
            {
                return Err(StateError::InvalidCatalog(
                    "policy-inventory-impact-conflict".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}

pub(crate) fn policy_digest(value: &impl Serialize) -> Result<String, StateError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| StateError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[derive(Serialize)]
pub(crate) struct CatalogIdentity<'a> {
    pub(crate) expected_context_id: &'a str,
    pub(crate) expected_authority_catalog_id: &'a str,
    pub(crate) claims: &'a [ClaimSpec],
    pub(crate) dependencies: &'a [DependencyFact],
    pub(crate) inventory_policies: &'a [InventoryPolicy],
    pub(crate) capability_requirements: &'a [CapabilityRequirement],
    pub(crate) runtime_metadata: &'a RuntimeMetadata,
    pub(crate) runtime_requirements: &'a [RuntimeRequirement],
    pub(crate) commands: &'a [CommandBinding],
    pub(crate) actions: &'a [ActionDefinition],
}

impl<'a> From<&'a DependencyActionSpec> for CatalogIdentity<'a> {
    fn from(spec: &'a DependencyActionSpec) -> Self {
        Self {
            expected_context_id: &spec.expected_context_id,
            expected_authority_catalog_id: &spec.expected_authority_catalog_id,
            claims: &spec.claims,
            dependencies: &spec.dependencies,
            inventory_policies: &spec.inventory_policies,
            capability_requirements: &spec.capability_requirements,
            runtime_metadata: &spec.runtime_metadata,
            runtime_requirements: &spec.runtime_requirements,
            commands: &spec.commands,
            actions: &spec.actions,
        }
    }
}
