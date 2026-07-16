use super::*;

impl FileAuthorityLedger {
    pub(crate) fn record_failure(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
        evidence: &ReservationFailureEvidence,
    ) -> Result<DurableWrite<()>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.record_failure(head, token, evidence);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (head, token, evidence);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }
}
