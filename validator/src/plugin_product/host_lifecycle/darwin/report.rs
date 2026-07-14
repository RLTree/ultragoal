use super::DarwinHostSnapshot;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DarwinHostTransactionDisposition {
    Applied,
    AlreadyConverged,
    Recovered,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DarwinHostTransactionReport {
    disposition: DarwinHostTransactionDisposition,
    plan_sha256: String,
    snapshot: DarwinHostSnapshot,
}

impl DarwinHostTransactionReport {
    pub(super) fn new(
        disposition: DarwinHostTransactionDisposition,
        plan_sha256: String,
        snapshot: DarwinHostSnapshot,
    ) -> Self {
        Self {
            disposition,
            plan_sha256,
            snapshot,
        }
    }

    pub const fn disposition(&self) -> DarwinHostTransactionDisposition {
        self.disposition
    }
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
    pub fn snapshot(&self) -> &DarwinHostSnapshot {
        &self.snapshot
    }
}
