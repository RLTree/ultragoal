use super::*;

#[path = "authority.rs"]
mod authority;
#[cfg(test)]
#[path = "cleanup_panic_tests.rs"]
mod cleanup_panic_tests;
#[cfg(test)]
#[path = "failure_matrix_tests.rs"]
mod failure_matrix_tests;
#[cfg(test)]
#[path = "failure_transition_tests.rs"]
mod failure_transition_tests;
#[cfg(test)]
#[path = "terminal_custody_tests.rs"]
mod terminal_custody_tests;

#[cfg(test)]
pub(super) use authority::observe_reservation;
pub(super) use authority::{
    ReservationAttempt, ReservationTerminal, observe_staged_transition, run_reserved,
};
