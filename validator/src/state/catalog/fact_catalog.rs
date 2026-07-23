use super::*;

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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionPriorityClass {
    ProtectedInvariant,
    ActiveTruthLoopTransition,
    FalsePassOrRejection,
    RepeatedCrossContextGap,
    BoundedExperiment,
    Speculative,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvidenceLedActionBinding {
    pub class: ActionPriorityClass,
    pub brief_digest: String,
    pub transition_id: Option<String>,
    pub transition_order: Option<u32>,
    pub active_trigger_ids: Vec<String>,
    pub parked_trigger_ids: Vec<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_led: Option<EvidenceLedActionBinding>,
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
    pub(crate) schema_version: &'static str,
    pub(crate) catalog_id: String,
    #[serde(skip)]
    pub(crate) spec_id: String,
    pub(crate) verification_modes:
        BTreeMap<String, crate::engineering_advisory::VerificationModeContract>,
    #[serde(skip)]
    pub(crate) authority: Option<PolicyPermit>,
    #[serde(flatten)]
    pub(crate) spec: DependencyActionSpec,
}

impl DependencyActionCatalog {
    /// Performs structural validation only. The returned catalog cannot
    /// initialize claim ceilings because it has no root-issued policy permit.
    pub fn from_untrusted_spec(mut spec: DependencyActionSpec) -> Result<Self, StateError> {
        super::super::limits::preflight_catalog(&spec)?;
        super::super::normalize::normalize(&mut spec);
        let problems = super::super::policy::validate_structure(&spec);
        if !problems.is_empty() {
            return Err(StateError::InvalidCatalog(problems.join(",")));
        }
        let empty_verification_modes =
            BTreeMap::<String, crate::engineering_advisory::VerificationModeContract>::new();
        let bytes = serde_json::to_vec(&(CatalogIdentity::from(&spec), &empty_verification_modes))
            .map_err(|error| StateError::Serialization(error.to_string()))?;
        if bytes.len() > super::super::limits::MAX_CATALOG_BYTES {
            return Err(StateError::ResourceLimit(
                "dependency/action catalog bytes".to_owned(),
            ));
        }
        let spec_id = format!("sha256:{:x}", Sha256::digest(bytes));
        Ok(Self {
            schema_version: "DependencyActionCatalog-v1",
            catalog_id: spec_id.clone(),
            spec_id,
            verification_modes: BTreeMap::new(),
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

    pub(crate) fn permit(&self) -> Option<&PolicyPermit> {
        self.authority.as_ref()
    }

    pub(crate) fn authorize(mut self, permit: PolicyPermit) -> Result<Self, StateError> {
        permit.verify_catalog(self.spec_id(), permit.permit_id())?;
        self.catalog_id = permit.permit_id().to_owned();
        self.authority = Some(permit);
        Ok(self)
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
