use super::super::catalog::{
    ActionDefinition, ActionKind, ClaimSpec, CommandBinding, DependencyActionCatalog,
    DependencyActionSpec, HostGoalObservation, RuntimeMetadata,
};
use super::super::policy_authority::PolicyAuthority;
use super::super::product_state::{
    AuthorityRequest, AuthorityRequirement, CeilingReduction, Repair, RepairTarget,
    RepairTargetKind, Scope,
};
use super::super::snapshot::BoundInputs;
use crate::context::EffectClass;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const CONTEXT_ID: &str = "sha256:context";
pub(super) const AUTHORITY_CATALOG_ID: &str = "sha256:authority";
pub(super) const CANDIDATE_ID: &str =
    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
pub(super) const CLAIM_REGISTRY_ID: &str =
    "sha256:67e81c4eabe87d16d816a3d7dad352dc21a4a1994dd83b6582e9e4ba5eaa61bc";

pub(super) fn inputs() -> BoundInputs {
    BoundInputs {
        context_id: CONTEXT_ID.to_owned(),
        authority_catalog_id: AUTHORITY_CATALOG_ID.to_owned(),
        authority_catalog_context_id: CONTEXT_ID.to_owned(),
        inventory_findings: Vec::new(),
        capabilities: BTreeMap::from([("git".to_owned(), true)]),
    }
}

pub(super) fn spec() -> DependencyActionSpec {
    DependencyActionSpec {
        expected_context_id: CONTEXT_ID.to_owned(),
        expected_authority_catalog_id: AUTHORITY_CATALOG_ID.to_owned(),
        claims: vec![ClaimSpec {
            claim_id: "CL-RUNTIME".to_owned(),
            maximum_dimensions: vec![
                "product".to_owned(),
                "runtime".to_owned(),
                "source".to_owned(),
            ],
        }],
        dependencies: Vec::new(),
        inventory_policies: Vec::new(),
        capability_requirements: Vec::new(),
        runtime_metadata: RuntimeMetadata::default(),
        runtime_requirements: Vec::new(),
        commands: vec![
            CommandBinding {
                command_id: "inspect-json".to_owned(),
                argv: vec![
                    "ultragoal".to_owned(),
                    "--json".to_owned(),
                    "inspect".to_owned(),
                ],
                effect: EffectClass::Read,
            },
            CommandBinding {
                command_id: "check".to_owned(),
                argv: vec!["ultragoal".to_owned(), "check".to_owned()],
                effect: EffectClass::Read,
            },
        ],
        actions: Vec::new(),
        host_goal: HostGoalObservation::default(),
    }
}

pub(super) fn catalog(spec: DependencyActionSpec) -> DependencyActionCatalog {
    let observed_codes = spec
        .inventory_policies
        .iter()
        .map(|policy| policy.code.clone())
        .collect();
    catalog_for_codes(spec, observed_codes).unwrap()
}

pub(super) fn catalog_for_codes(
    spec: DependencyActionSpec,
    observed_codes: BTreeSet<String>,
) -> Result<DependencyActionCatalog, super::super::product_state::StateError> {
    catalog_for_binding(
        spec,
        CONTEXT_ID,
        AUTHORITY_CATALOG_ID,
        CANDIDATE_ID,
        observed_codes,
    )
}

pub(super) fn catalog_for_binding(
    spec: DependencyActionSpec,
    context_id: &str,
    authority_catalog_id: &str,
    candidate_id: &str,
    observed_codes: BTreeSet<String>,
) -> Result<DependencyActionCatalog, super::super::product_state::StateError> {
    let authority =
        PolicyAuthority::from_adopted_claim_registry(CLAIM_REGISTRY_ID, spec.claims.clone(), spec)?;
    authority.issue_for_test(
        context_id,
        authority_catalog_id,
        candidate_id,
        &observed_codes,
    )
}

pub(super) fn reduction(dimensions: &[&str]) -> Vec<CeilingReduction> {
    vec![CeilingReduction {
        claim_id: "CL-RUNTIME".to_owned(),
        dimensions: dimensions.iter().map(|item| (*item).to_owned()).collect(),
    }]
}

pub(super) fn repair(id: &str, authority: AuthorityRequirement, effect: EffectClass) -> Repair {
    Repair {
        repair_id: id.to_owned(),
        target: RepairTarget {
            kind: RepairTargetKind::Dependency,
            id: id.to_owned(),
        },
        summary: format!("Repair root cause {id}"),
        effect,
        authority,
        rerun_command_id: "inspect-json".to_owned(),
        authority_decision: match authority {
            AuthorityRequirement::External => Some(AuthorityRequest {
                target: id.to_owned(),
                consequence: "Dependent claim ceiling remains lowered".to_owned(),
                reversible: true,
                accepted_loss_required: None,
            }),
            AuthorityRequirement::HumanDestructive => Some(AuthorityRequest {
                target: id.to_owned(),
                consequence: "The named target would be irreversibly changed".to_owned(),
                reversible: false,
                accepted_loss_required: Some("Explicit loss acceptance is required".to_owned()),
            }),
            _ => None,
        },
        invalidates_evidence: BTreeSet::from([format!("evidence:{id}")]),
        projected_ceiling_after_reverification: Vec::new(),
    }
}

pub(super) fn command(id: &str, repair_id: &str, priority: u32) -> ActionDefinition {
    ActionDefinition {
        action_id: id.to_owned(),
        priority,
        kind: ActionKind::Command,
        repair_id: repair_id.to_owned(),
        requires_dependencies: Vec::new(),
        required_capabilities: Vec::new(),
        effect: EffectClass::Read,
        authority: AuthorityRequirement::None,
        command_id: Some("check".to_owned()),
        authority_request: None,
        evidence_led: None,
    }
}

pub(super) fn scope(surface: &str) -> Scope {
    Scope {
        surface: surface.to_owned(),
        relative_path: None,
    }
}
