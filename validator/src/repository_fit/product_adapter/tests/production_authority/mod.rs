#![cfg(target_vendor = "apple")]

use super::super::authority::{
    RepositoryFitApplyNonce, RepositoryFitAuthorityStore, RepositoryFitTrustedClock,
    after_effect_before_terminal_for_test, after_effect_start_before_apply_for_test,
    after_reservation_for_test, configure_effects_for_test, execute_prepared_apply,
    parse_recovery_intent, prepare_recovery_intent, recover_prepared_apply,
};
use super::super::catalog::CANONICAL_TEMPLATES;
use super::super::ledger::{
    FileRepositoryFitLedger, LedgerErrorId, RecoveryTargetRow, RecoveryTargetSpec,
    RepositoryFitLedgerState, ReservationDecision, ReservationRequest,
    before_atomic_publish_for_test, before_existing_open_for_test, before_lock_acquire_for_test,
    canonical_recovery_intent_bytes,
};
use super::super::root_permit::managed_ancestor_contract_for_ledger_test;
use super::super::{AdapterErrorId, plan_target, prepare_apply_request, verify_target};
use super::scenario::{git_status, snapshot};
use crate::context::{BuildRequest, LiveContext};
use crate::repository_fit::{
    FitAdapterError, FitReader, LocalRepository, PreparedFitApply, digest, inspect_target,
};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex, mpsc};
use std::time::{Duration, Instant};

#[path = "abrupt_process_exit_after_effect_with_target_substitution_recovers_as_ambiguous.rs"]
mod abrupt_process_exit_after_effect_with_target_substitution_recovers_as_ambiguous;
#[path = "authority_fixtures.rs"]
mod authority_fixtures;
#[path = "failed_terminal_validation_retains_recovery_authority_until_reconciled.rs"]
mod failed_terminal_validation_retains_recovery_authority_until_reconciled;
#[path = "inspect_plan_verify_and_verify_as_apply_are_recursively_zero_write.rs"]
mod inspect_plan_verify_and_verify_as_apply_are_recursively_zero_write;
#[path = "live_effect_owner_holds_process_lock_through_mutation_and_terminal.rs"]
mod live_effect_owner_holds_process_lock_through_mutation_and_terminal;
#[path = "missing_recovery_ledger_refuses_without_initializing_authority_or_writing_target.rs"]
mod missing_recovery_ledger_refuses_without_initializing_authority_or_writing_target;
#[path = "production_mutation_grant_has_one_private_mint_in_the_sealed_authority.rs"]
mod production_mutation_grant_has_one_private_mint_in_the_sealed_authority;
#[path = "same_session_stale_head_replay_and_terminal_substitution_fail_closed.rs"]
mod same_session_stale_head_replay_and_terminal_substitution_fail_closed;
#[path = "scenario_fixture.rs"]
mod scenario_fixture;
#[path = "stale_target_and_invalid_clock_refuse_before_authority_store_write.rs"]
mod stale_target_and_invalid_clock_refuse_before_authority_store_write;
#[path = "subprocess_reservation_entrypoint.rs"]
mod subprocess_reservation_entrypoint;
#[path = "wait_for_path.rs"]
mod wait_for_path;
#[path = "whole_root_rename_with_exact_leaf_postimage_cannot_recover_as_committed.rs"]
mod whole_root_rename_with_exact_leaf_postimage_cannot_recover_as_committed;

pub(crate) use authority_fixtures::*;
pub(crate) use scenario_fixture::*;
pub(crate) use wait_for_path::*;
