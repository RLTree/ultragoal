use super::StateEngine;
use super::adopted_claims::{RootClaimStage, stage_root, stage_target};
use super::adopted_registry::load_adopted_claims;
use super::catalog::{
    ActionDefinition, ActionKind, ClaimSpec, CommandBinding, DependencyActionCatalog,
    DependencyActionSpec, DependencyFact, DependencyStatus, FactAuthority, HostGoalObservation,
    InventoryPolicy, RuntimeMetadata,
};
use super::policy_authority::PolicyAuthority;
use super::product_state::{
    AuthorityRequirement, CeilingReduction, ProductState, Repair, RepairTarget, RepairTargetKind,
    Scope, StateError,
};
use crate::context::{EffectClass, LiveContext};
use crate::inventory::AuthorityCatalog;
use std::collections::BTreeSet;

pub(crate) fn derive_adopted(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
) -> Result<ProductState, StateError> {
    let policy = issue_adopted(context, authority_catalog)?;
    StateEngine::derive(context, authority_catalog, &policy)
}

pub(crate) fn issue_adopted(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
) -> Result<DependencyActionCatalog, StateError> {
    let loaded = load_adopted_claims()?;
    let claim_registry_id = loaded.claim_registry_sha256.clone();
    let staged = stage_target(
        context,
        authority_catalog,
        &loaded.registry,
        loaded.claim_registry_sha256.clone(),
        loaded.contract_manifest_sha256,
        loaded.handoff_sha256,
    )?;
    let claims = loaded
        .registry
        .claims
        .into_iter()
        .map(|claim| ClaimSpec {
            claim_id: claim.claim_id,
            maximum_dimensions: vec![claim.allowed_ceiling_on_pass],
        })
        .collect::<Vec<_>>();
    let spec = live_spec(context, authority_catalog, &claims, staged.stage_id()?);
    PolicyAuthority::from_adopted_claim_registry(claim_registry_id, claims, spec)?
        .issue(context, authority_catalog)
}

pub(crate) fn stage_root_claims(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
) -> Result<RootClaimStage, StateError> {
    let loaded = load_adopted_claims()?;
    stage_root(
        context,
        authority_catalog,
        &loaded.registry,
        loaded.claim_registry_sha256,
        loaded.contract_manifest_sha256,
        loaded.handoff_sha256,
    )
}

fn live_spec(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
    claims: &[ClaimSpec],
    staged_reconciliation_id: &str,
) -> DependencyActionSpec {
    let reductions = all_reductions(claims);
    let codes = authority_catalog
        .findings()
        .iter()
        .map(|finding| finding.code.clone())
        .collect::<BTreeSet<_>>();
    let inventory_policies = codes
        .iter()
        .map(|code| inventory_policy(code, &reductions))
        .collect();
    let mut actions = codes
        .iter()
        .enumerate()
        .map(|(index, code)| ActionDefinition {
            action_id: format!("plan-inventory-{code}"),
            priority: index as u32 + 100,
            kind: ActionKind::Command,
            repair_id: format!("repair-inventory-{code}"),
            requires_dependencies: Vec::new(),
            required_capabilities: Vec::new(),
            effect: EffectClass::Read,
            authority: AuthorityRequirement::Root,
            command_id: Some("migrate-plan".to_owned()),
            authority_request: None,
            evidence_led: None,
        })
        .collect::<Vec<_>>();
    let inception =
        crate::product_inception::bind_actions(context, authority_catalog, &mut actions, &codes);
    let mut dependencies = vec![DependencyFact {
        dependency_id: "reconciliation-kernel".to_owned(),
        observation_id: staged_reconciliation_id.to_owned(),
        status: DependencyStatus::Missing,
        authority: FactAuthority::DirectProbe,
        scope: Scope {
            surface: "claim-graph".to_owned(),
            relative_path: None,
        },
        cause: "canonical claim reconciliation is not yet supplied".to_owned(),
        repair: Some(repair(
            "repair-reconciliation-kernel",
            RepairTargetKind::Dependency,
            "reconciliation-kernel",
            "Implement the canonical reconciliation kernel",
        )),
        ceiling_reductions: reductions.clone(),
    }];
    if matches!(
        inception,
        crate::product_inception::RankingDisposition::InceptionRequired
    ) {
        dependencies.push(DependencyFact {
            dependency_id: "product-inception".to_owned(),
            observation_id: context.context_id().to_owned(),
            status: DependencyStatus::Missing,
            authority: FactAuthority::DirectProbe,
            scope: Scope {
                surface: "product-success-brief".to_owned(),
                relative_path: Some("PRODUCT_SUCCESS_BRIEF.json".to_owned()),
            },
            cause: "a current valid Product Success Brief v2 is required".to_owned(),
            repair: Some(repair(
                "repair-product-inception",
                RepairTargetKind::Dependency,
                "product-inception",
                "Inspect and repair the current Product Success Brief",
            )),
            ceiling_reductions: reductions.clone(),
        });
        actions.push(ActionDefinition {
            action_id: "inspect-product-inception".to_owned(),
            priority: 1,
            kind: ActionKind::Command,
            repair_id: "repair-product-inception".to_owned(),
            requires_dependencies: Vec::new(),
            required_capabilities: Vec::new(),
            effect: EffectClass::Read,
            authority: AuthorityRequirement::Root,
            command_id: Some("inspect-inception".to_owned()),
            authority_request: None,
            evidence_led: None,
        });
    }
    DependencyActionSpec {
        expected_context_id: context.context_id().to_owned(),
        expected_authority_catalog_id: authority_catalog.catalog_id().to_owned(),
        claims: claims.to_vec(),
        dependencies,
        inventory_policies,
        capability_requirements: Vec::new(),
        runtime_metadata: RuntimeMetadata::default(),
        runtime_requirements: Vec::new(),
        commands: vec![
            CommandBinding {
                command_id: "inspect-inception".to_owned(),
                argv: vec![
                    "ultragoal".to_owned(),
                    "--json".to_owned(),
                    "inspect".to_owned(),
                    "inception".to_owned(),
                ],
                effect: EffectClass::Read,
            },
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
                command_id: "migrate-plan".to_owned(),
                argv: vec![
                    "ultragoal".to_owned(),
                    "--json".to_owned(),
                    "migrate".to_owned(),
                    "plan".to_owned(),
                ],
                effect: EffectClass::Read,
            },
        ],
        actions,
        host_goal: HostGoalObservation::default(),
    }
}

fn inventory_policy(code: &str, reductions: &[CeilingReduction]) -> InventoryPolicy {
    InventoryPolicy {
        code: code.to_owned(),
        scope_surface: "source-authority".to_owned(),
        repair: repair(
            &format!("repair-inventory-{code}"),
            RepairTargetKind::Source,
            "source-authority",
            &format!("Plan the canonical migration for inventory finding {code}"),
        ),
        ceiling_reductions: reductions.to_vec(),
    }
}

fn repair(id: &str, kind: RepairTargetKind, target: &str, summary: &str) -> Repair {
    Repair {
        repair_id: id.to_owned(),
        target: RepairTarget {
            kind,
            id: target.to_owned(),
        },
        summary: summary.to_owned(),
        effect: EffectClass::Read,
        authority: AuthorityRequirement::Root,
        rerun_command_id: "inspect-json".to_owned(),
        authority_decision: None,
        invalidates_evidence: BTreeSet::from(["state-policy".to_owned()]),
        projected_ceiling_after_reverification: Vec::new(),
    }
}

fn all_reductions(claims: &[ClaimSpec]) -> Vec<CeilingReduction> {
    claims
        .iter()
        .map(|claim| CeilingReduction {
            claim_id: claim.claim_id.clone(),
            dimensions: claim.maximum_dimensions.iter().cloned().collect(),
        })
        .collect()
}
