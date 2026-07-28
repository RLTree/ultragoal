use super::{CheckResult, LawFinding};
use std::path::Path;

const CHECK_ID: &str = "namespace-progressive-disclosure";

pub(super) fn run(root: &Path) -> (Vec<CheckResult>, Vec<LawFinding>) {
    let findings = crate::audit::namespace::law::current_root_failures(root)
        .into_iter()
        .map(|detail| LawFinding {
            check_id: CHECK_ID.to_owned(),
            detail,
        })
        .collect::<Vec<_>>();
    let checks = vec![CheckResult {
        check_id: CHECK_ID.to_owned(),
        status: if findings.is_empty() { "pass" } else { "fail" },
        finding_count: findings.len(),
    }];
    (checks, findings)
}
