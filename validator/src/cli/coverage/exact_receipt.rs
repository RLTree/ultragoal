use serde_json::Value;
use std::path::Path;

const REQUIRED_BLOCKED: &[&str] = &[
    "completion",
    "package_readiness",
    "review_readiness",
    "release_readiness",
    "final_packet_correctness",
    "update_goal_eligibility",
    "app_registry_or_reviewer_exposure",
];

pub(crate) struct FailureCodes {
    pub(crate) target_dir_missing: &'static str,
    pub(crate) target_dir_not_isolated: &'static str,
    pub(crate) target_digest_mismatch: &'static str,
    pub(crate) not_exact: &'static str,
    pub(crate) claim_ceiling_not_complete: &'static str,
    pub(crate) supported_claim_missing: Option<&'static str>,
    pub(crate) blocked_claim_missing_prefix: Option<&'static str>,
}

pub(crate) const EVIDENCE_CODES: FailureCodes = FailureCodes {
    target_dir_missing: "coverage_target_dir_missing",
    target_dir_not_isolated: "coverage_target_dir_not_isolated",
    target_digest_mismatch: "coverage_target_digest_mismatch",
    not_exact: "coverage_not_exact_100",
    claim_ceiling_not_complete: "coverage_claim_ceiling_not_complete",
    supported_claim_missing: Some("coverage_supported_claim_missing"),
    blocked_claim_missing_prefix: Some("coverage_missing_blocked_claim"),
};

pub(crate) const TRANSACTION_CODES: FailureCodes = FailureCodes {
    target_dir_missing: "cli_control_plane_transaction_coverage_target_dir_missing",
    target_dir_not_isolated: "cli_control_plane_transaction_coverage_target_dir_not_isolated",
    target_digest_mismatch: "cli_control_plane_transaction_coverage_digest_mismatch",
    not_exact: "cli_control_plane_transaction_coverage_not_exact_100",
    claim_ceiling_not_complete: "cli_control_plane_transaction_coverage_claim_ceiling_not_complete",
    supported_claim_missing: None,
    blocked_claim_missing_prefix: None,
};

pub(crate) fn claim_failures(
    root: &Path,
    value: &Value,
    expected: &str,
    codes: &FailureCodes,
) -> Vec<String> {
    let mut out = Vec::new();
    target_dir_failures(root, value, codes, &mut out);
    if value
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        != Some(expected)
    {
        out.push(codes.target_digest_mismatch.to_string());
    }
    if value.pointer("/coverage/percent").and_then(Value::as_f64) != Some(100.0)
        || !value
            .get("uncovered_records")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    {
        out.push(codes.not_exact.to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str)
        != Some("supports_complete_coverage_claim")
    {
        out.push(codes.claim_ceiling_not_complete.to_string());
    }
    if let Some(code) = codes.supported_claim_missing
        && !array_contains(value, "supported_claim_classes", "complete_coverage")
    {
        out.push(code.to_string());
    }
    if let Some(prefix) = codes.blocked_claim_missing_prefix {
        for claim in REQUIRED_BLOCKED {
            if !array_contains(value, "blocked_claim_classes", claim) {
                out.push(format!("{prefix}:{claim}"));
            }
        }
    }
    out
}

fn target_dir_failures(root: &Path, value: &Value, codes: &FailureCodes, out: &mut Vec<String>) {
    let target_dir = value
        .get("coverage_target_dir")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match super::target_dir::status(root, target_dir) {
        super::target_dir::TargetDirStatus::Missing => {
            out.push(codes.target_dir_missing.to_string())
        }
        super::target_dir::TargetDirStatus::NotIsolated => {
            out.push(codes.target_dir_not_isolated.to_string())
        }
        super::target_dir::TargetDirStatus::Isolated => {}
    }
}

fn array_contains(value: &Value, key: &str, needle: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(needle)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exact_receipt_reports_unisolated_coverage_target_dir() {
        let root = std::env::current_dir().expect("cwd");
        let receipt = json!({
            "coverage_target_dir": "target",
            "target_revision": {"value": crate::digest::ZERO},
            "coverage": {"percent": 100.0},
            "uncovered_records": [],
            "claim_ceiling": "supports_complete_coverage_claim",
            "supported_claim_classes": ["complete_coverage"],
            "blocked_claim_classes": REQUIRED_BLOCKED
        });

        let failures = claim_failures(&root, &receipt, crate::digest::ZERO, &EVIDENCE_CODES);

        assert!(
            failures
                .iter()
                .any(|failure| failure == "coverage_target_dir_not_isolated"),
            "{failures:?}"
        );
    }
}
