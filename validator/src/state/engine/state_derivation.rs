use super::*;

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

pub(crate) fn derive_bound(
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

pub(crate) fn initial_ceilings(
    spec: &super::super::catalog::DependencyActionSpec,
) -> BTreeMap<String, ClaimCeiling> {
    let mut ceilings = BTreeMap::new();
    for claim in &spec.claims {
        ceilings.entry(claim.claim_id.clone()).or_insert_with(|| {
            ClaimCeiling::from_dimensions(claim.claim_id.clone(), claim.maximum_dimensions.clone())
        });
    }
    ceilings
}

pub(crate) fn all_reductions(ceilings: &BTreeMap<String, ClaimCeiling>) -> Vec<CeilingReduction> {
    ceilings
        .values()
        .map(|ceiling| CeilingReduction {
            claim_id: ceiling.claim_id().to_owned(),
            dimensions: ceiling.dimensions().clone(),
        })
        .collect()
}

pub(crate) fn policy_finding(
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
