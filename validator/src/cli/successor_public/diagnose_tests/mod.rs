use super::repository_fixture::{Repository, tree};
use super::{execute_invocation, read_context};
use crate::cli::successor::{OutputMode, ParseOutcome, parse_args};
use crate::inventory::InventoryBuilder;
use crate::observability::{EventStore, SemanticEvent};
use crate::state::{ProductState, derive_adopted};
use serde_json::Value;
use std::fs;

#[path = "receipt_only_event_cannot_be_promoted_to_a_cause.rs"]
mod receipt_only_event_cannot_be_promoted_to_a_cause;
#[path = "selected_finding_joins_state_repair_to_explicit_local_cause_without_writes.rs"]
mod selected_finding_joins_state_repair_to_explicit_local_cause_without_writes;

pub(crate) use receipt_only_event_cannot_be_promoted_to_a_cause::*;
