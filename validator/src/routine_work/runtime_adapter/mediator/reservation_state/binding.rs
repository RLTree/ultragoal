use super::*;

pub(super) struct ReservationBinding {
    protocol_id: String,
    grant_id: String,
    recovery_marker: String,
    prior_recovery_marker: Option<String>,
}

impl ReservationBinding {
    pub(super) fn new(
        protocol_id: String,
        grant_id: String,
        recovery_marker: String,
        prior_recovery_marker: Option<String>,
    ) -> Self {
        Self {
            protocol_id,
            grant_id,
            recovery_marker,
            prior_recovery_marker,
        }
    }

    pub(super) fn protocol_id(&self) -> &String {
        &self.protocol_id
    }

    pub(super) fn grant_id(&self) -> &String {
        &self.grant_id
    }

    pub(super) fn recovery_marker(&self) -> &String {
        &self.recovery_marker
    }

    pub(super) fn prior_recovery_marker(&self) -> Option<&String> {
        self.prior_recovery_marker.as_ref()
    }

    pub(super) fn expected_ambiguity(&self, started: bool) -> Option<&String> {
        if started {
            Some(&self.recovery_marker)
        } else {
            self.prior_recovery_marker.as_ref()
        }
    }

    pub(super) fn failure_evidence(
        &self,
        primary: FailureEvidence,
        process_cleanup: CleanupEvidence,
        staged_cleanup: CleanupEvidence,
        started: bool,
    ) -> ReservationFailureEvidence {
        ReservationFailureEvidence {
            schema_version: RESERVATION_FAILURE_SCHEMA.to_owned(),
            protocol_id: self.protocol_id.clone(),
            grant_id: self.grant_id.clone(),
            recovery_marker: self.recovery_marker.clone(),
            primary,
            process_cleanup,
            staged_cleanup,
            disposition: if started {
                ReservationFailureDisposition::StartedPending
            } else {
                ReservationFailureDisposition::ReservedPending
            },
        }
    }

    pub(super) fn validates_failure(
        &self,
        evidence: &ReservationFailureEvidence,
        started: bool,
    ) -> bool {
        evidence.protocol_id == self.protocol_id
            && evidence.grant_id == self.grant_id
            && evidence.recovery_marker == self.recovery_marker
            && evidence.disposition
                == if started {
                    ReservationFailureDisposition::StartedPending
                } else {
                    ReservationFailureDisposition::ReservedPending
                }
    }
}
