use super::catalog::DependencyActionSpec;
use super::product_state::{CeilingReduction, Repair};
use serde::Serialize;

pub(crate) fn normalize(spec: &mut DependencyActionSpec) {
    for claim in &mut spec.claims {
        claim.maximum_dimensions.sort();
        claim.maximum_dimensions.dedup();
    }
    sort_canonical(&mut spec.claims);
    for fact in &mut spec.dependencies {
        normalize_reductions(&mut fact.ceiling_reductions);
        if let Some(repair) = &mut fact.repair {
            normalize_repair(repair);
        }
    }
    sort_canonical(&mut spec.dependencies);
    for policy in &mut spec.inventory_policies {
        normalize_reductions(&mut policy.ceiling_reductions);
        normalize_repair(&mut policy.repair);
    }
    sort_canonical(&mut spec.inventory_policies);
    for requirement in &mut spec.capability_requirements {
        normalize_reductions(&mut requirement.ceiling_reductions);
        normalize_repair(&mut requirement.repair);
    }
    sort_canonical(&mut spec.capability_requirements);
    for requirement in &mut spec.runtime_requirements {
        normalize_reductions(&mut requirement.ceiling_reductions);
        normalize_repair(&mut requirement.repair);
    }
    sort_canonical(&mut spec.runtime_requirements);
    sort_canonical(&mut spec.commands);
    for action in &mut spec.actions {
        action.requires_dependencies.sort();
        action.requires_dependencies.dedup();
        action.required_capabilities.sort();
        action.required_capabilities.dedup();
    }
    sort_canonical(&mut spec.actions);
}

fn normalize_repair(repair: &mut Repair) {
    sort_canonical(&mut repair.projected_ceiling_after_reverification);
}

fn normalize_reductions(reductions: &mut Vec<CeilingReduction>) {
    sort_canonical(reductions);
    reductions.dedup();
}

fn sort_canonical<T: Serialize>(items: &mut [T]) {
    items.sort_by_cached_key(|item| {
        serde_json::to_vec(item).expect("owned state catalog values always serialize")
    });
}
