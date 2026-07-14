use super::ceiling::ClaimCeiling;
use super::product_state::{Finding, Repair};
use std::collections::BTreeMap;

pub(crate) fn apply_reductions(
    ceilings: &mut BTreeMap<String, ClaimCeiling>,
    findings: &[Finding],
) {
    for finding in findings {
        for reduction in &finding.ceiling_reductions {
            if let Some(ceiling) = ceilings.get_mut(&reduction.claim_id) {
                ceiling.lower(&reduction.dimensions, finding.finding_id.clone());
            }
        }
    }
}

pub(crate) fn unique_repairs(findings: &[Finding]) -> Vec<Repair> {
    let mut repairs = BTreeMap::new();
    for finding in findings {
        repairs
            .entry(finding.repair.repair_id.clone())
            .or_insert_with(|| finding.repair.clone());
    }
    repairs.into_values().collect()
}
