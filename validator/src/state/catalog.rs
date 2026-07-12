use super::policy_authority::PolicyPermit;
use super::types::{
    AuthorityRequest, AuthorityRequirement, CeilingReduction, Repair, Scope, StateError,
};
use crate::context::EffectClass;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub use super::provenance::{
    HostGoalObservation, HostGoalStatus, RuntimeField, RuntimeMetadata, RuntimeRequirement,
    RuntimeSource, RuntimeValue,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FactAuthority {
    DirectProbe,
    LiveContext,
    AuthorityCatalog,
    ExternalAuthority,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyStatus {
    Satisfied,
    Missing,
    BlockedAuthority,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencyFact {
    pub dependency_id: String,
    pub observation_id: String,
    pub status: DependencyStatus,
    pub authority: FactAuthority,
    pub scope: Scope,
    pub cause: String,
    pub repair: Option<Repair>,
    pub ceiling_reductions: Vec<CeilingReduction>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClaimSpec {
    pub claim_id: String,
    pub maximum_dimensions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InventoryPolicy {
    pub code: String,
    pub scope_surface: String,
    pub repair: Repair,
    pub ceiling_reductions: Vec<CeilingReduction>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CapabilityRequirement {
    pub capability: String,
    pub scope: Scope,
    pub repair: Repair,
    pub ceiling_reductions: Vec<CeilingReduction>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionKind {
    Command,
    AuthorityRequest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CommandBinding {
    pub command_id: String,
    pub argv: Vec<String>,
    pub effect: EffectClass,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActionDefinition {
    pub action_id: String,
    pub priority: u32,
    pub kind: ActionKind,
    pub repair_id: String,
    pub requires_dependencies: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub effect: EffectClass,
    pub authority: AuthorityRequirement,
    pub command_id: Option<String>,
    pub authority_request: Option<AuthorityRequest>,
}

/// A caller-authored policy proposal. It is untrusted until the root-owned
/// policy authority binds it to the adopted claim registry and live inputs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencyActionSpec {
    pub expected_context_id: String,
    pub expected_authority_catalog_id: String,
    pub claims: Vec<ClaimSpec>,
    pub dependencies: Vec<DependencyFact>,
    pub inventory_policies: Vec<InventoryPolicy>,
    pub capability_requirements: Vec<CapabilityRequirement>,
    pub runtime_metadata: RuntimeMetadata,
    pub runtime_requirements: Vec<RuntimeRequirement>,
    pub commands: Vec<CommandBinding>,
    pub actions: Vec<ActionDefinition>,
    pub host_goal: HostGoalObservation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencyActionCatalog {
    schema_version: &'static str,
    catalog_id: String,
    #[serde(skip)]
    spec_id: String,
    #[serde(skip)]
    authority: Option<PolicyPermit>,
    #[serde(flatten)]
    spec: DependencyActionSpec,
}

impl DependencyActionCatalog {
    /// Performs structural validation only. The returned catalog cannot
    /// initialize claim ceilings because it has no root-issued policy permit.
    pub fn from_untrusted_spec(mut spec: DependencyActionSpec) -> Result<Self, StateError> {
        super::limits::preflight_catalog(&spec)?;
        super::normalize::normalize(&mut spec);
        let problems = super::policy::validate_structure(&spec);
        if !problems.is_empty() {
            return Err(StateError::InvalidCatalog(problems.join(",")));
        }
        let bytes = serde_json::to_vec(&CatalogIdentity::from(&spec))
            .map_err(|error| StateError::Serialization(error.to_string()))?;
        if bytes.len() > super::limits::MAX_CATALOG_BYTES {
            return Err(StateError::ResourceLimit(
                "dependency/action catalog bytes".to_owned(),
            ));
        }
        let spec_id = format!("sha256:{:x}", Sha256::digest(bytes));
        Ok(Self {
            schema_version: "DependencyActionCatalog-v1",
            catalog_id: spec_id.clone(),
            spec_id,
            authority: None,
            spec,
        })
    }

    pub fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub(crate) fn spec_id(&self) -> &str {
        &self.spec_id
    }

    pub(crate) fn spec(&self) -> &DependencyActionSpec {
        &self.spec
    }

    pub(super) fn permit(&self) -> Option<&PolicyPermit> {
        self.authority.as_ref()
    }

    pub(super) fn authorize(mut self, permit: PolicyPermit) -> Result<Self, StateError> {
        permit.verify_catalog(self.spec_id(), permit.permit_id())?;
        self.catalog_id = permit.permit_id().to_owned();
        self.authority = Some(permit);
        Ok(self)
    }

    pub(crate) fn recompute_spec_id(&self) -> Result<String, StateError> {
        let bytes = serde_json::to_vec(&CatalogIdentity::from(&self.spec))
            .map_err(|error| StateError::Serialization(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    #[cfg(test)]
    pub(crate) fn test_permit_from(&mut self, other: &Self) {
        self.authority = other.authority.clone();
    }

    #[cfg(test)]
    pub(crate) fn test_mutate_permit(&mut self, field: &str, value: &str) {
        self.authority.as_mut().unwrap().test_mutate(field, value);
    }
}

pub(crate) fn canonical_claims(mut claims: Vec<ClaimSpec>) -> Result<Vec<ClaimSpec>, StateError> {
    if claims.is_empty() || claims.len() > super::limits::MAX_CLAIMS {
        return Err(StateError::InvalidCatalog(
            "policy-claim-set-invalid".to_owned(),
        ));
    }
    let mut ids = BTreeSet::new();
    for claim in &mut claims {
        if !super::limits::valid_id(&claim.claim_id)
            || claim.maximum_dimensions.is_empty()
            || !ids.insert(claim.claim_id.clone())
            || claim
                .maximum_dimensions
                .iter()
                .any(|dimension| !super::limits::valid_id(dimension))
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
struct CatalogIdentity<'a> {
    expected_context_id: &'a str,
    expected_authority_catalog_id: &'a str,
    claims: &'a [ClaimSpec],
    dependencies: &'a [DependencyFact],
    inventory_policies: &'a [InventoryPolicy],
    capability_requirements: &'a [CapabilityRequirement],
    runtime_metadata: &'a RuntimeMetadata,
    runtime_requirements: &'a [RuntimeRequirement],
    commands: &'a [CommandBinding],
    actions: &'a [ActionDefinition],
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
