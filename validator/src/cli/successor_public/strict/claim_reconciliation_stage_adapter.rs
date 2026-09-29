use super::{CheckResult, LawFinding};
use crate::context::LiveContext;
use crate::inventory::InventoryBuilder;

const CHECK_ID: &str = "claim-reconciliation-stage";

pub(super) fn run(context: &LiveContext) -> (Vec<CheckResult>, Vec<LawFinding>) {
    let result = InventoryBuilder::new(context)
        .build()
        .map_err(|_| "authority-inventory-unavailable")
        .and_then(|inventory| {
            crate::state::stage_root_claims(context, &inventory)
                .map(|stage| stage.stage_id().to_owned())
                .map_err(|_| "root-claim-stage-refused")
        });
    let findings = match result {
        Ok(stage_id) if stage_id.starts_with("sha256:") && stage_id.len() == 71 => Vec::new(),
        Ok(_) => vec![LawFinding {
            check_id: CHECK_ID.to_owned(),
            detail: "root-claim-stage-identity-invalid".to_owned(),
        }],
        Err(detail) => vec![LawFinding {
            check_id: CHECK_ID.to_owned(),
            detail: detail.to_owned(),
        }],
    };
    let checks = vec![CheckResult {
        check_id: CHECK_ID.to_owned(),
        status: if findings.is_empty() { "pass" } else { "fail" },
        finding_count: findings.len(),
    }];
    (checks, findings)
}
