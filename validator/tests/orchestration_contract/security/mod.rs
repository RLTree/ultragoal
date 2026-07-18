use crate::orchestration::*;
use crate::orchestration_fixture::*;

include!("failures_never_echo_attacker_controlled_input.rs");

include!("destructive_effect_cannot_hide_under_external_bounded_safety.rs");
