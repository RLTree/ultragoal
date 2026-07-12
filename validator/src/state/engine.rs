use super::catalog::{DependencyActionCatalog, DependencyStatus};
use super::ceiling::ClaimCeiling;
use super::findings::{
    authority_name, contradiction_repair, dependency_code, dependency_severity, finding,
    policy_repair,
};
use super::snapshot::BoundInputs;
use super::types::{
    CeilingReduction, Finding, FindingSeverity, FindingSource, NextActionKind, ProductGoalState,
    ProductState, Scope, StateError,
};
use super::{identity, next, observations, policy_authority, reduce};
use crate::context::LiveContext;
use crate::inventory::{AuthorityCatalog, FindingSeverity as InventorySeverity};
use std::collections::{BTreeMap, BTreeSet};

pub struct StateEngine;

impl StateEngine {
    pub fn derive(
        context: &LiveContext,
        authority_catalog: &AuthorityCatalog,
        dependency_actions: &DependencyActionCatalog,
    ) -> Result<ProductState, StateError> {
        context
            .revalidate()
            .map_err(|error| StateError::StaleContext(error.to_string()))?;
        policy_authority::verify_live(dependency_actions, context, authority_catalog)?;
        let state = derive_bound(
            BoundInputs::from_live(context, authority_catalog),
            dependency_actions,
        )?;
        context
            .revalidate()
            .map_err(|error| StateError::StaleContext(error.to_string()))?;
        policy_authority::verify_live(dependency_actions, context, authority_catalog)?;
        Ok(state)
    }
}

pub(super) fn derive_bound(
    inputs: BoundInputs,
    catalog: &DependencyActionCatalog,
) -> Result<ProductState, StateError> {
    inputs.validate()?;
    let inventory_codes = inputs
        .inventory_findings
        .iter()
        .map(|finding| finding.code.clone())
        .collect();
    policy_authority::verify_bound(
        catalog,
        &inputs.context_id,
        &inputs.authority_catalog_id,
        &inputs.authority_catalog_context_id,
        None,
        &inventory_codes,
    )?;
    let spec = catalog.spec();
    let mut ceilings = initial_ceilings(spec);
    let all_reductions = all_reductions(&ceilings);
    let mut findings = Vec::new();
    let mut fatal = Vec::new();
    inventory_findings(&inputs, catalog, &mut findings, &mut fatal);
    let dependency_states = dependency_findings(catalog, &mut findings, &mut fatal);
    observations::capability_findings(&inputs, catalog, &mut findings);
    observations::runtime_findings(catalog, &mut findings);
    fatal.sort();
    fatal.dedup();
    for code in &fatal {
        findings.push(policy_finding(catalog, code, &all_reductions));
    }
    findings.sort_by(|a, b| (&a.code, &a.finding_id).cmp(&(&b.code, &b.finding_id)));
    findings.dedup_by(|a, b| a.finding_id == b.finding_id);
    reduce::apply_reductions(&mut ceilings, &findings);
    if !fatal.is_empty() {
        for ceiling in ceilings.values_mut() {
            ceiling.remove_all("invalid-state-policy");
        }
    }
    let claim_ceilings = ceilings.into_values().collect::<Vec<_>>();
    let repairs = reduce::unique_repairs(&findings);
    let next_action = next::select(
        &spec.commands,
        &spec.actions,
        &findings,
        &dependency_states,
        &inputs.capabilities,
        !fatal.is_empty(),
    );
    let product_goal = match next_action.kind {
        NextActionKind::NoOp => ProductGoalState::NoAction,
        NextActionKind::NoLegalRoute => ProductGoalState::Blocked,
        NextActionKind::AuthorityRequest => ProductGoalState::AwaitingAuthority,
        NextActionKind::Command
            if findings
                .iter()
                .any(|item| item.severity == FindingSeverity::Blocked) =>
        {
            ProductGoalState::Blocked
        }
        NextActionKind::Command => ProductGoalState::Operating,
    };
    let host_goal = spec.host_goal.clone();
    let state_id = identity::state_id(identity::StateIdentity {
        schema_version: "ProductState-v1",
        context_id: &inputs.context_id,
        authority_catalog_id: &inputs.authority_catalog_id,
        dependency_action_catalog_id: catalog.catalog_id(),
        product_goal,
        runtime_metadata: &spec.runtime_metadata,
        findings: &findings,
        repairs: &repairs,
        claim_ceilings: &claim_ceilings,
        next_action: &next_action,
    })?;
    Ok(ProductState {
        schema_version: "ProductState-v1",
        state_id,
        context_id: inputs.context_id,
        authority_catalog_id: inputs.authority_catalog_id,
        dependency_action_catalog_id: catalog.catalog_id().to_owned(),
        product_goal,
        host_goal,
        runtime_metadata: spec.runtime_metadata.clone(),
        findings,
        repairs,
        claim_ceilings,
        next_action,
    })
}

fn initial_ceilings(spec: &super::catalog::DependencyActionSpec) -> BTreeMap<String, ClaimCeiling> {
    let mut ceilings = BTreeMap::new();
    for claim in &spec.claims {
        ceilings.entry(claim.claim_id.clone()).or_insert_with(|| {
            ClaimCeiling::from_dimensions(claim.claim_id.clone(), claim.maximum_dimensions.clone())
        });
    }
    ceilings
}

fn all_reductions(ceilings: &BTreeMap<String, ClaimCeiling>) -> Vec<CeilingReduction> {
    ceilings
        .values()
        .map(|ceiling| CeilingReduction {
            claim_id: ceiling.claim_id().to_owned(),
            dimensions: ceiling.dimensions().clone(),
        })
        .collect()
}

fn policy_finding(
    catalog: &DependencyActionCatalog,
    code: &str,
    reductions: &[CeilingReduction],
) -> Finding {
    finding(
        "invalid-state-policy",
        FindingSeverity::Error,
        FindingSource::StatePolicy {
            catalog_id: catalog.catalog_id().to_owned(),
        },
        Scope {
            surface: "authority-kernel".to_owned(),
            relative_path: None,
        },
        BTreeSet::new(),
        format!("dependency/action catalog defect: {code}"),
        policy_repair(code),
        reductions.to_vec(),
    )
}

fn inventory_findings(
    inputs: &BoundInputs,
    catalog: &DependencyActionCatalog,
    output: &mut Vec<Finding>,
    fatal: &mut Vec<String>,
) {
    for observation in &inputs.inventory_findings {
        let policies = catalog
            .spec()
            .inventory_policies
            .iter()
            .filter(|policy| policy.code == observation.code)
            .collect::<Vec<_>>();
        if policies.len() != 1 {
            fatal.push(format!(
                "inventory-policy-count:{}:{}",
                observation.code,
                policies.len()
            ));
            continue;
        }
        let policy = policies[0];
        output.push(finding(
            observation.code.clone(),
            match observation.severity {
                InventorySeverity::Error => FindingSeverity::Error,
                InventorySeverity::Warning => FindingSeverity::Warning,
                InventorySeverity::Info => FindingSeverity::Info,
            },
            FindingSource::AuthorityCatalog {
                catalog_id: inputs.authority_catalog_id.clone(),
                code: observation.code.clone(),
            },
            Scope {
                surface: policy.scope_surface.clone(),
                relative_path: observation.relative_path.clone(),
            },
            observation.entry_id.iter().cloned().collect(),
            observation.cause.clone(),
            policy.repair.clone(),
            policy.ceiling_reductions.clone(),
        ));
    }
}

fn dependency_findings(
    catalog: &DependencyActionCatalog,
    output: &mut Vec<Finding>,
    fatal: &mut Vec<String>,
) -> BTreeMap<String, Option<DependencyStatus>> {
    let mut groups: BTreeMap<&str, Vec<&super::catalog::DependencyFact>> = BTreeMap::new();
    for fact in &catalog.spec().dependencies {
        groups.entry(&fact.dependency_id).or_default().push(fact);
    }
    let mut states = BTreeMap::new();
    for (dependency_id, facts) in groups {
        let statuses = facts
            .iter()
            .map(|fact| fact.status)
            .collect::<BTreeSet<_>>();
        if statuses.len() > 1 {
            states.insert(dependency_id.to_owned(), None);
            let reductions = facts
                .iter()
                .flat_map(|fact| fact.ceiling_reductions.clone())
                .collect();
            let cause = facts
                .iter()
                .map(|fact| {
                    format!(
                        "{}:{}:{:?}",
                        fact.observation_id,
                        authority_name(fact.authority),
                        fact.status
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            output.push(finding(
                "contradictory-dependency",
                FindingSeverity::Error,
                FindingSource::DependencyCatalog {
                    catalog_id: catalog.catalog_id().to_owned(),
                    observation_id: "multiple".to_owned(),
                },
                Scope {
                    surface: "dependency-graph".to_owned(),
                    relative_path: None,
                },
                BTreeSet::from([dependency_id.to_owned()]),
                cause,
                contradiction_repair(dependency_id),
                reductions,
            ));
            continue;
        }
        let status = *statuses.iter().next().expect("nonempty dependency facts");
        states.insert(dependency_id.to_owned(), Some(status));
        if status == DependencyStatus::Satisfied {
            continue;
        }
        for fact in facts {
            let Some(repair) = fact.repair.clone() else {
                fatal.push(format!("missing-repair:{dependency_id}"));
                continue;
            };
            output.push(finding(
                dependency_code(status),
                dependency_severity(status),
                FindingSource::DependencyCatalog {
                    catalog_id: catalog.catalog_id().to_owned(),
                    observation_id: fact.observation_id.clone(),
                },
                fact.scope.clone(),
                BTreeSet::from([dependency_id.to_owned()]),
                fact.cause.clone(),
                repair,
                fact.ceiling_reductions.clone(),
            ));
        }
    }
    states
}
