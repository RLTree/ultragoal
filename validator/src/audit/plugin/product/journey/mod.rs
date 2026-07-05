use serde_json::Value;
use std::path::Path;

mod record;

use record::{FIT_REPO_RECEIPT, JourneyEvidenceRef, PluginProductJourneyReceipt};

pub(crate) fn failures(root: &Path, value: &Value) -> Vec<String> {
    failures_with_digest(root, value, crate::package::inventory::package_digest(root))
}

pub(crate) fn failures_with_candidate(
    root: &Path,
    value: &Value,
    target_digest: &str,
) -> Vec<String> {
    failures_with_digest(root, value, Ok(target_digest.to_string()))
}

fn failures_with_digest(
    root: &Path,
    value: &Value,
    current: Result<String, String>,
) -> Vec<String> {
    let receipt = PluginProductJourneyReceipt::from_value(value);
    if receipt.journey_step_count < 10 {
        return vec!["plugin_product_journey_incomplete".to_string()];
    }
    let mut out = evidence_count_failures(&receipt);
    out.extend(authority_failures(&receipt, current));
    if receipt.claim_ceiling != "package_static_fixture_only" {
        out.push("plugin_product_journey_claim_ceiling_missing".to_string());
    }
    if receipt.generated_at.contains("2026-06-24T00:00:00Z") {
        out.push("plugin_product_journey_placeholder_timestamp".to_string());
    }
    out.extend(evidence_ref_failures(root, &receipt));
    out
}

fn evidence_count_failures(receipt: &PluginProductJourneyReceipt) -> Vec<String> {
    if receipt.evidence.len() < 3
        || receipt
            .evidence
            .iter()
            .filter(|item| item.path != FIT_REPO_RECEIPT)
            .count()
            < 2
    {
        vec!["plugin_product_journey_evidence_incomplete".to_string()]
    } else {
        Vec::new()
    }
}

fn authority_failures(
    receipt: &PluginProductJourneyReceipt,
    current: Result<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    if receipt.status != "pass" {
        out.push("plugin_product_journey_status_not_pass".to_string());
    }
    if receipt.target_revision_kind != "package_digest" {
        out.push("plugin_product_journey_target_revision_not_package_digest".to_string());
    }
    match current {
        Ok(current) if receipt.target_revision_value == current => {}
        Ok(_) => out.push("plugin_product_journey_target_digest_mismatch".to_string()),
        Err(err) => out.push(format!(
            "plugin_product_journey_target_digest_unavailable:{err}"
        )),
    }
    out
}

fn evidence_ref_failures(root: &Path, receipt: &PluginProductJourneyReceipt) -> Vec<String> {
    let mut out = Vec::new();
    for item in &receipt.evidence {
        out.extend(evidence_failure(root, item, "plugin journey evidence"));
    }
    if let Some(item) = &receipt.error_path_evidence {
        out.extend(evidence_failure(root, item, "plugin journey error path"));
    } else {
        out.push("plugin_product_journey_error_path_missing".to_string());
    }
    out
}

fn evidence_failure(root: &Path, item: &JourneyEvidenceRef, label: &str) -> Vec<String> {
    let result = item
        .artifact
        .as_ref()
        .map_err(ToString::to_string)
        .and_then(|artifact| artifact.validate(root, label));
    match result {
        Ok(()) => Vec::new(),
        Err(err) => vec![format!("plugin_product_journey_evidence_invalid:{err}")],
    }
}
