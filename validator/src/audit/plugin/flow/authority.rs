use crate::audit::plugin::product::cohesion::manifest::PluginCohesionManifest;
use std::collections::BTreeSet;

const REQUIRED_EDGES: &[&str] = &[
    "harness-ultragoal:fit-repo->validation_artifacts/harness/fit-repo-receipt.json",
    "harness-ultragoal:fit-repo->templates/.harness/coverage-manifest.json",
    "templates/scripts/check-coverage-full->coverage-proof-policy",
    "harness-ultragoal:product-fitness-gate->validation_artifacts/harness/product-fitness-receipt.json",
    "templates/scripts/check-agent-standards->agent-standards-enforcement",
];

pub(crate) fn failures(flow: &PluginCohesionManifest) -> Vec<String> {
    let mut out = Vec::new();
    let declared = flow
        .validator_checks
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    for check in crate::audit::contract::CHECK_IDS
        .iter()
        .copied()
        .filter(|id| !crate::contract_check_ids::is_retained_compatibility_id(id))
    {
        if !declared.contains(check) {
            out.push(format!("plugin_flow_validator_check_missing:{check}"));
        }
    }
    let edge_set = flow.edge_keys().into_iter().collect::<BTreeSet<_>>();
    for edge in REQUIRED_EDGES {
        if !edge_set.contains(*edge) {
            out.push(format!("plugin_flow_required_edge_missing:{edge}"));
        }
    }
    out.extend(flow_receipt_failures(flow, &edge_set));
    out
}

#[cfg(test)]
pub(crate) fn failures_from_value(flow: &serde_json::Value) -> Vec<String> {
    failures(&PluginCohesionManifest::from_value(flow))
}

fn flow_receipt_failures(
    flow: &PluginCohesionManifest,
    edge_set: &BTreeSet<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    if flow.flows.is_empty() {
        return vec!["plugin_flow_completion_receipt_missing:flows".to_string()];
    }
    for row in &flow.flows {
        let id = if row.id.is_empty() {
            "<unknown>"
        } else {
            &row.id
        };
        if row.completion_receipt.is_empty() {
            out.push(format!("plugin_flow_completion_receipt_missing:{id}"));
        }
        for edge in &row.required_edges {
            if !edge_set.contains(edge) {
                out.push(format!("plugin_flow_required_edge_missing:{id}:{edge}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::failures_from_value;
    use serde_json::json;

    #[test]
    fn current_flow_excludes_retained_checks_and_keeps_source_card_freshness() {
        let failures = failures_from_value(&json!({
            "validator_checks":["source-card-freshness"],
            "edges":[],
            "flows":[]
        }));
        for retained in [
            "research-source-authority-article-to-law-integration",
            "source-card-freshness-ceiling",
        ] {
            assert!(
                !failures.contains(&format!("plugin_flow_validator_check_missing:{retained}")),
                "{retained}: {failures:?}"
            );
        }
        assert!(
            !failures
                .contains(&"plugin_flow_validator_check_missing:source-card-freshness".to_string()),
            "{failures:?}"
        );
        assert!(
            failures.contains(
                &"plugin_flow_validator_check_missing:agent-standards-enforcement".to_string()
            ),
            "{failures:?}"
        );
    }
}
