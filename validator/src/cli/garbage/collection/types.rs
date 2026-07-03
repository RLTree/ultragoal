pub(crate) const GC_RECEIPT_SCHEMA: &str = "harness-ultragoal.workspace-gc-receipt.v1";
pub(crate) const GC_POLICY_VERSION: &str = "2026-06-26.rust-runtime-policy.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GarbageOperation {
    Plan,
    DryRun,
    Apply,
    Verify,
}

impl GarbageOperation {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::DryRun => "dry_run",
            Self::Apply => "apply",
            Self::Verify => "verify",
        }
    }
}
