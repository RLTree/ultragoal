use super::repository_fixture::{Repository, tree};
use super::{
    MAX_PUBLIC_OUTPUT, compatibility_workspace_context, current_read_context,
    current_workspace_context, execute_invocation, execute_invocation_with_home, parse_public,
    public_output_allowed, read_context,
};
use crate::cli::successor::{OutputMode, ParseOutcome, ParsedInvocation, parse_args};
use std::fs;
use std::path::{Path, PathBuf};

fn disposable_home(label: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("ultragoal-{label}-{}-home", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    fs::canonicalize(home).unwrap()
}

#[path = "accepted_observe_query_reads_current_local_events_without_writes.rs"]
mod accepted_observe_query_reads_current_local_events_without_writes;
mod current_authority;
mod evaluation;
mod evaluation_run;
#[path = "fit_apply_runs_the_public_production_route_and_retires_recovery_state.rs"]
mod fit_apply_runs_the_public_production_route_and_retires_recovery_state;
mod inspection;
#[path = "local_state_fit_plan_is_public_and_confined.rs"]
mod local_state_fit_plan_is_public_and_confined;
mod migration_plan;
#[path = "public_output_limit_is_inclusive_and_fail_closed.rs"]
mod public_output_limit_is_inclusive_and_fail_closed;
#[path = "repository_fit_authority_quarantine_apply.rs"]
mod repository_fit_authority_quarantine_apply;
#[path = "routine_configuration_fit_plan_is_public_and_confined.rs"]
mod routine_configuration_fit_plan_is_public_and_confined;
#[path = "routine_diagnosis_without_inventory.rs"]
mod routine_diagnosis_without_inventory;
#[path = "unavailable_context_is_stable_and_does_not_echo_input.rs"]
mod unavailable_context_is_stable_and_does_not_echo_input;
