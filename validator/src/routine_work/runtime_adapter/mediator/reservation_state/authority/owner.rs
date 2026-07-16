#[path = "owner/attempt.rs"]
mod attempt;
#[path = "owner/registry.rs"]
mod registry;

pub(in super::super::super) use attempt::{
    ReservationAttempt, ReservationTerminal, observe_staged_transition, run_reserved,
};
#[cfg(test)]
pub(in super::super::super) use registry::observe;
