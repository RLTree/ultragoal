#[cfg(test)]
mod admission_race_tests;
mod append;
mod observations;

pub(in super::super) use append::append_routine_terminal;
pub(in super::super) use observations::routine_observations_from_events;
#[allow(unused_imports)]
pub(in super::super) use observations::{RoutineTerminalEvent, terminal_event_id};
