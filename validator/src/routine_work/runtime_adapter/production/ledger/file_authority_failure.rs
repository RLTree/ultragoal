use super::*;

impl FileAuthorityLedger {
    pub(crate) fn record_failure(
        &self,
        token: &ReservationToken,
        evidence: &ReservationFailureEvidence,
    ) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.record_failure(token, evidence);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, evidence);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    #[cfg(all(test, target_vendor = "apple"))]
    pub(crate) fn test_failure_records(
        &self,
        binding: &AuthorityBinding,
    ) -> Result<Vec<ReservationFailureEvidence>, RoutineError> {
        self.inner.test_failure_records(binding)
    }
}
