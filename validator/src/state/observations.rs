use super::catalog::DependencyActionCatalog;
use super::findings::{FindingInput, finding};
use super::product_state::{Finding, FindingSeverity, FindingSource};
use super::snapshot::BoundInputs;
use std::collections::BTreeSet;

pub(crate) fn capability_findings(
    inputs: &BoundInputs,
    catalog: &DependencyActionCatalog,
    output: &mut Vec<Finding>,
) {
    for requirement in &catalog.spec().capability_requirements {
        if inputs
            .capabilities
            .get(&requirement.capability)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }
        output.push(finding(FindingInput {
            code: "unsupported-capability".to_owned(),
            severity: FindingSeverity::Warning,
            source: FindingSource::LiveContext {
                context_id: inputs.context_id.clone(),
            },
            scope: requirement.scope.clone(),
            dependency_ids: BTreeSet::from([format!("capability:{}", requirement.capability)]),
            cause: format!(
                "required capability {} is not exposed",
                requirement.capability
            ),
            repair: requirement.repair.clone(),
            reductions: requirement.ceiling_reductions.clone(),
        }));
    }
}

pub(crate) fn runtime_findings(catalog: &DependencyActionCatalog, output: &mut Vec<Finding>) {
    for requirement in &catalog.spec().runtime_requirements {
        let value = catalog.spec().runtime_metadata.value(requirement.field);
        if value.is_some_and(|item| {
            !item.value().trim().is_empty() && !item.exposed_source().trim().is_empty()
        }) {
            continue;
        }
        output.push(finding(FindingInput {
            code: "unverified-runtime-metadata".to_owned(),
            severity: FindingSeverity::Warning,
            source: FindingSource::RuntimeMetadata {
                field: requirement.field.name().to_owned(),
            },
            scope: requirement.scope.clone(),
            dependency_ids: BTreeSet::from([format!("runtime:{}", requirement.field.name())]),
            cause: format!(
                "{} was not exposed by Codex or runtime metadata",
                requirement.field.name()
            ),
            repair: requirement.repair.clone(),
            reductions: requirement.ceiling_reductions.clone(),
        }));
    }
}
