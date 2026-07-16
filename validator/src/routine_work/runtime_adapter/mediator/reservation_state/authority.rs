use super::*;

#[path = "authority/binding.rs"]
mod binding;
#[path = "authority/durable_binding.rs"]
mod durable_binding;
#[path = "authority/failure_observation.rs"]
mod failure_observation;
#[path = "authority/owner.rs"]
mod owner;

pub(in super::super) use owner::{
    ReservationAttempt, ReservationTerminal, observe_staged_transition, run_reserved,
};

#[cfg(test)]
pub(in super::super) struct ReservationObservation {
    pub(in super::super) active_grant: Option<String>,
    pub(in super::super) recovery_marker: Option<String>,
    pub(in super::super) failure: Option<ReservationFailureEvidence>,
    pub(in super::super) non_durable_witness: Option<String>,
}

#[cfg(test)]
pub(in super::super) fn observe_reservation(
    protocol: &str,
    failure_marker: Option<&str>,
    artifact_digest: Option<&str>,
) -> ReservationObservation {
    owner::observe(protocol, failure_marker, artifact_digest)
}
