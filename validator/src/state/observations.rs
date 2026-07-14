use super::catalog::DependencyActionCatalog;
use super::findings::finding;
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
        output.push(finding(
            "unsupported-capability",
            FindingSeverity::Warning,
            FindingSource::LiveContext {
                context_id: inputs.context_id.clone(),
            },
            requirement.scope.clone(),
            BTreeSet::from([format!("capability:{}", requirement.capability)]),
            format!(
                "required capability {} is not exposed",
                requirement.capability
            ),
            requirement.repair.clone(),
            requirement.ceiling_reductions.clone(),
        ));
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
        output.push(finding(
            "unverified-runtime-metadata",
            FindingSeverity::Warning,
            FindingSource::RuntimeMetadata {
                field: requirement.field.name().to_owned(),
            },
            requirement.scope.clone(),
            BTreeSet::from([format!("runtime:{}", requirement.field.name())]),
            format!(
                "{} was not exposed by Codex or runtime metadata",
                requirement.field.name()
            ),
            requirement.repair.clone(),
            requirement.ceiling_reductions.clone(),
        ));
    }
}
