use super::repository_fixture::{Repository, tree};
use super::{
    MAX_PUBLIC_OUTPUT, execute_invocation, execute_invocation_with_home, parse_public,
    public_output_allowed, read_context,
};
use crate::cli::successor::{OutputMode, ParseOutcome, ParsedInvocation, parse_args};
use crate::observability::{EventStore, SemanticEvent};
use std::fs;
use std::path::Path;

#[path = "accepted_observe_query_reads_current_local_events_without_writes.rs"]
mod accepted_observe_query_reads_current_local_events_without_writes;
#[path = "fit_apply_runs_the_public_production_route_and_retires_recovery_state.rs"]
mod fit_apply_runs_the_public_production_route_and_retires_recovery_state;
#[path = "inspect_capabilities_blocks_aliased_authority_roots_without_writes.rs"]
mod inspect_capabilities_blocks_aliased_authority_roots_without_writes;
#[path = "inspect_capabilities_rejects_ambient_home_without_authority_io.rs"]
mod inspect_capabilities_rejects_ambient_home_without_authority_io;
#[path = "public_output_limit_is_inclusive_and_fail_closed.rs"]
mod public_output_limit_is_inclusive_and_fail_closed;
#[path = "unavailable_context_is_stable_and_does_not_echo_input.rs"]
mod unavailable_context_is_stable_and_does_not_echo_input;
