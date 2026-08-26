use super::super::{CheckpointBinding, RoutineStateMigrationAdmission, RoutineStateQuarantinePlan};
use super::*;
use crate::routine_work::{
    ReservedRecoveryEffectEvidence, ReservedRecoveryOwnerObservation, RoutineCustodyCapability,
    RoutineReservedRecoveryAssessment,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::ffi::OsStrExt;

const AUTHORITY_ENTRIES: &[&str] = &[
    "routine-authority.key",
    "routine-authority.lock",
    "routine-authority.state",
];
const MAX_AUTHORITY_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_EVENT_ENTRY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CHECKPOINT_ENTRY_BYTES: u64 = 16 * 1024;
const MAX_LOCK_ENTRY_BYTES: u64 = 1024;
const STALE_RESERVED_HEAD_CEILING: &str = "noncurrent_value_only_historical_head_unproven";
const QUARANTINE_PLAN_NEXT_ACTION: &str = "preserve all bytes; persist the exact ultragoal --json diagnose JSON unchanged at a relative in-repository path, independently review its deterministic quarantine_plan, then run ultragoal --json migrate apply --plan <relative-diagnosis-record> --accept-plan <quarantine_plan.plan_id>; migrate plan emits a separate ProductMigration projection and is not this command's input";
const QUARANTINE_PLAN_SCHEMA: &str = "RoutineStateQuarantinePlan-v5";
const ABANDONMENT_PLAN_SCHEMA: &str = "RoutineStateQuarantinePlan-v6";
const ABANDONMENT_PLAN_NEXT_ACTION: &str = "review the declared loss of legacy continuity, reuse, and recovery; persist this exact ultragoal --json migrate abandon-plan record unchanged at a relative in-repository path, then run ultragoal --json migrate apply --plan <relative-abandonment-record> --accept-plan <quarantine_plan.plan_id> --approve-retirement";
const ABANDONMENT_CONSEQUENCE_BINDING: &[u8] = b"\0legacy-history-continuity-abandoned\0legacy-authority-not-imported\0previous-reuse-and-recovery-unavailable\0declared-target-local-routine-outputs-may-reexecute";
const IGNORED_PARENT_SNAPSHOT_PREFIX: &str = "owner_parent_ignored/";
const QUARANTINE_STRATEGY: &str =
    "stage_fresh_v8_then_atomic_exchange_finalize_legacy_and_publish_settlement_receipt";
const QUARANTINE_APPLY_CAPABILITY: &str = "available_exact_record_only";
const QUARANTINE_PLAN_OPERATIONS: &[&str] = &[
    "acquire_parent_migration_lock_and_legacy_adapter_lock_then_revalidate_exact_source_inventory",
    "stage_and_verify_fresh_v8_owner_with_plan_bound_transition_marker_without_importing_legacy_authority",
    "atomically_exchange_legacy_source_owner_with_verified_fresh_v8_stage",
    "fsync_owner_parent_after_atomic_exchange",
    "finalize_swapped_legacy_stage_to_bound_quarantine_sibling",
    "fsync_owner_parent_after_quarantine_finalize",
    "verify_active_fresh_v8_transition_and_unchanged_quarantine_inventory",
    "stage_and_fsync_exact_plan_bound_settlement_receipt",
    "atomically_publish_settlement_receipt_and_fsync_active_fresh_v8_owner",
    "verify_permanent_pending_and_settled_receipts_with_active_fresh_v8_and_unchanged_quarantine",
];
const QUARANTINE_ROLLBACK: &str = "before_swap: preserve the verified plan-bound unpublished fresh-v8 stage for exact replay while the legacy source remains active; after_swap: never exchange back or delete either owner, finalize only the unchanged legacy stage to the bound quarantine sibling, retain the permanent pending record, and publish only an exact plan-bound settlement receipt; pending-only, staged-receipt, or malformed settlement evidence preserves every observed name and holds";
const QUARANTINE_PLAN_STRATEGY_BINDING: &[u8] =
    b"\0stage-fresh-v8-then-atomic-exchange-finalize-legacy-and-publish-settlement-receipt";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InventoryEntry {
    identity: Identity,
    content_sha256: Option<[u8; 32]>,
}

pub(crate) fn assess_migration_admission(
    home: &Path,
    binding: CheckpointBinding<'_>,
) -> Result<Option<RoutineStateMigrationAdmission>, HostFailure> {
    assess_admission(home, binding, LegacyAdmissionMode::Migration)
}

pub(crate) fn assess_abandonment_admission(
    home: &Path,
    binding: CheckpointBinding<'_>,
) -> Result<Option<RoutineStateMigrationAdmission>, HostFailure> {
    assess_admission(home, binding, LegacyAdmissionMode::ExplicitAbandonment)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LegacyAdmissionMode {
    Migration,
    ExplicitAbandonment,
}

fn assess_admission(
    home: &Path,
    binding: CheckpointBinding<'_>,
    mode: LegacyAdmissionMode,
) -> Result<Option<RoutineStateMigrationAdmission>, HostFailure> {
    let home = host_state::open_home(home)?;
    let codex = match home.open_child(STATE_COMPONENTS[0]) {
        Ok(value) => value,
        Err(HostFailure::Unavailable) => return Ok(None),
        Err(error) => return Err(error),
    };
    let state_root = match codex.open_child(STATE_COMPONENTS[1]) {
        Ok(value) => value,
        Err(HostFailure::Unavailable) => return Ok(None),
        Err(error) => return Err(error),
    };
    let harness = match state_root.open_child(STATE_COMPONENTS[2]) {
        Ok(value) => value,
        Err(HostFailure::Unavailable) => return Ok(None),
        Err(error) => return Err(error),
    };
    let parent_state_lock = ParentStateLock::shared(&harness)?;
    let transition_entries = transition_entries(&harness)?;
    let state = match harness.open_child(STATE_COMPONENTS[3]) {
        Ok(value) => value,
        Err(HostFailure::Unavailable) if !transition_entries.is_empty() => {
            return Ok(Some(transition_hold()));
        }
        Err(HostFailure::Unavailable) => return Ok(None),
        Err(error) => return Err(error),
    };

    if state.stat(STATE_FORMAT_NAME)?.is_some() {
        if transition_entries.contains(BOOTSTRAP_STAGE) {
            return Ok(Some(transition_hold()));
        }
        match quarantine_transition::classify_present(&state) {
            Ok(Some((accepted, quarantine_transition::SettlementState::Settled)))
                if transition_entries.len() == 1
                    && transition_entries.contains(&accepted.quarantine_owner) => {}
            Ok(None) if transition_entries.is_empty() => {}
            _ => return Ok(Some(transition_hold())),
        }
        return assess_versioned(home, state, parent_state_lock).map(Some);
    }
    if !transition_entries.is_empty() {
        return Ok(Some(transition_hold()));
    }
    assess_format_absent_with_mode(&harness, state, binding, STATE_COMPONENTS[3], &[], mode)
        .map(Some)
}

fn transition_entries(owner_parent: &AnchoredDirectory) -> Result<BTreeSet<String>, HostFailure> {
    Ok(owner_parent
        .entry_names()?
        .into_iter()
        .filter(|name| name == BOOTSTRAP_STAGE || name.starts_with("routine-public.quarantine-"))
        .collect())
}

pub(super) fn has_transition_evidence(
    owner_parent: &AnchoredDirectory,
) -> Result<bool, HostFailure> {
    let entries = transition_entries(owner_parent)?;
    if entries
        .iter()
        .any(|name| name.starts_with("routine-public.quarantine-"))
    {
        return Ok(true);
    }
    if !entries.contains(BOOTSTRAP_STAGE) {
        return Ok(false);
    }
    let stage = match owner_parent.open_child(BOOTSTRAP_STAGE) {
        Ok(stage) => stage,
        Err(_) => return Ok(true),
    };
    quarantine_transition::is_present(&stage)
}

fn transition_hold() -> RoutineStateMigrationAdmission {
    RoutineStateMigrationAdmission {
        status: "quarantine_transition_preserve_and_hold",
        format_status: "quarantine_or_bootstrap_transition",
        legacy_singleton_count: 0,
        canonical_continuation_count: 0,
        event_journal_count: 0,
        history_relation: "transition_requires_exact_accepted_plan",
        next_action: "preserve the quarantine and bootstrap evidence; retry only through the exact accepted quarantine plan",
        reserved_recovery: None,
        quarantine_plan: None,
    }
}

fn assess_versioned(
    home: AnchoredDirectory,
    state: AnchoredDirectory,
    parent_state_lock: ParentStateLock,
) -> Result<RoutineStateMigrationAdmission, HostFailure> {
    let format = state.open_regular(STATE_FORMAT_NAME, libc::O_RDONLY, 0o600);
    let Ok(format) = format else {
        return Ok(unsafe_hold("unknown_or_unsafe"));
    };
    let mut bytes = Vec::new();
    format
        .take((STATE_FORMAT_BYTES.len() + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| HostFailure::Invalid)?;
    if bytes != STATE_FORMAT_BYTES {
        return Ok(RoutineStateMigrationAdmission {
            status: "unknown_format_hold",
            format_status: "unsupported",
            legacy_singleton_count: 0,
            canonical_continuation_count: 0,
            event_journal_count: 0,
            history_relation: "not_applicable",
            next_action: "preserve the store and use a source revision with an explicit reader for this format",
            reserved_recovery: None,
            quarantine_plan: None,
        });
    }
    match host_state::open_existing_state(home, state, parent_state_lock) {
        Ok(current) => {
            let counts = counts(&current.adapter)?;
            Ok(RoutineStateMigrationAdmission {
                status: "already_current",
                format_status: "routine_host_state_v8",
                legacy_singleton_count: counts.0,
                canonical_continuation_count: counts.1,
                event_journal_count: counts.2,
                history_relation: "already_current",
                next_action: "no migration is required",
                reserved_recovery: None,
                quarantine_plan: None,
            })
        }
        Err(HostFailure::Busy) => Ok(stale_writer_hold("routine_host_state_v8")),
        Err(_) => Ok(unsafe_hold("routine_host_state_v8")),
    }
}

pub(super) fn assess_format_absent(
    owner_parent: &AnchoredDirectory,
    state: AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    observed_owner: &str,
    ignored_parent_entries: &[&str],
) -> Result<RoutineStateMigrationAdmission, HostFailure> {
    assess_format_absent_with_mode(
        owner_parent,
        state,
        binding,
        observed_owner,
        ignored_parent_entries,
        LegacyAdmissionMode::Migration,
    )
}

pub(super) fn assess_format_absent_for_abandonment(
    owner_parent: &AnchoredDirectory,
    state: AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    observed_owner: &str,
    ignored_parent_entries: &[&str],
) -> Result<RoutineStateMigrationAdmission, HostFailure> {
    assess_format_absent_with_mode(
        owner_parent,
        state,
        binding,
        observed_owner,
        ignored_parent_entries,
        LegacyAdmissionMode::ExplicitAbandonment,
    )
}

fn assess_format_absent_with_mode(
    owner_parent: &AnchoredDirectory,
    state: AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    observed_owner: &str,
    ignored_parent_entries: &[&str],
    mode: LegacyAdmissionMode,
) -> Result<RoutineStateMigrationAdmission, HostFailure> {
    if state.entry_names()?
        != [AUTHORITY_DIRECTORY, ADAPTER_DIRECTORY, LAUNCH_DIRECTORY]
            .into_iter()
            .map(str::to_owned)
            .collect()
    {
        return Ok(unsafe_hold("absent"));
    }
    let authority = match state.open_child(AUTHORITY_DIRECTORY) {
        Ok(value) => value,
        Err(_) => return Ok(unsafe_hold("absent")),
    };
    let adapter = match state.open_child(ADAPTER_DIRECTORY) {
        Ok(value) => value,
        Err(_) => return Ok(unsafe_hold("absent")),
    };
    let launch = match state.open_child(LAUNCH_DIRECTORY) {
        Ok(value) => value,
        Err(_) => return Ok(unsafe_hold("absent")),
    };
    if !launch.entry_names()?.is_empty() || !valid_authority_shape(&authority)? {
        return Ok(unsafe_hold("absent"));
    }
    let lock = match adapter.open_regular(LOCK_NAME, libc::O_RDONLY, 0o600) {
        Ok(value) => value,
        Err(_) => return Ok(unsafe_hold("absent")),
    };
    let lock = match ProcessLock::acquire(lock) {
        Ok(value) => value.0,
        Err(HostFailure::Busy) => return Ok(stale_writer_hold("absent")),
        Err(_) => return Ok(unsafe_hold("absent")),
    };
    if host_state::read_lock_marker(&lock)? != LOCK_MARKER {
        return Ok(unsafe_hold("absent"));
    }
    if !basic_legacy_shape(&adapter)? {
        return Ok(unsafe_hold("absent"));
    }

    let before = inventory(
        owner_parent,
        &state,
        &authority,
        &adapter,
        &launch,
        observed_owner,
        ignored_parent_entries,
    )?;
    let (singleton_count, continuation_count, event_count) = counts(&adapter)?;
    if singleton_count == 1 && continuation_count > 0 {
        let relation = match reconcile_mixed_history(&authority, &adapter, &launch, binding) {
            Ok(value) => value,
            Err(_) => return Ok(unsafe_hold("absent")),
        };
        let after = inventory(
            owner_parent,
            &state,
            &authority,
            &adapter,
            &launch,
            observed_owner,
            ignored_parent_entries,
        )?;
        if before != after {
            return Ok(stale_writer_hold("absent"));
        }
        if relation.terminal_events != TerminalEventAssessment::Complete {
            return if mode == LegacyAdmissionMode::ExplicitAbandonment {
                terminal_event_abandonment_admission(
                    singleton_count,
                    continuation_count,
                    event_count,
                    relation.terminal_events,
                    &before,
                    binding.target(),
                )
            } else {
                Ok(terminal_event_hold(
                    singleton_count,
                    continuation_count,
                    event_count,
                    relation.terminal_events,
                ))
            };
        }
        return reconciled_hold(
            singleton_count,
            continuation_count,
            event_count,
            relation,
            &before,
            binding.target(),
            mode,
        );
    }
    if validate_legacy(&adapter).is_err() {
        return Ok(unsafe_hold("absent"));
    }
    let after = inventory(
        owner_parent,
        &state,
        &authority,
        &adapter,
        &launch,
        observed_owner,
        ignored_parent_entries,
    )?;
    if before != after {
        return Ok(stale_writer_hold("absent"));
    }
    let quarantine_plan = quarantine_plan(&before, "single_history_only", binding.target())?;
    Ok(RoutineStateMigrationAdmission {
        status: "migration_candidate_requires_approval",
        format_status: "absent_legacy",
        legacy_singleton_count: singleton_count,
        canonical_continuation_count: continuation_count,
        event_journal_count: event_count,
        history_relation: "single_history_only",
        next_action: QUARANTINE_PLAN_NEXT_ACTION,
        reserved_recovery: None,
        quarantine_plan: Some(quarantine_plan),
    })
}

fn basic_legacy_shape(adapter: &AnchoredDirectory) -> Result<bool, HostFailure> {
    for name in adapter.entry_names()? {
        if name == LOCK_NAME {
            continue;
        }
        if name == CONTINUITY_CHECKPOINT_NAME {
            if adapter.open_regular(&name, libc::O_RDONLY, 0o600).is_err() {
                return Ok(false);
            }
            continue;
        }
        if name == CONTINUITY_DIRECTORY_NAME {
            let Ok(directory) = adapter.open_child(&name) else {
                return Ok(false);
            };
            for record in directory.entry_names()? {
                let Some(digest) = record
                    .strip_prefix("routine-continuation-")
                    .and_then(|value| value.strip_suffix(".json"))
                else {
                    return Ok(false);
                };
                if digest.len() != 64
                    || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                    || directory
                        .open_regular(&record, libc::O_RDONLY, 0o600)
                        .is_err()
                {
                    return Ok(false);
                }
            }
            continue;
        }
        if host_state::validate_event_leaf_name(&name).is_err()
            || adapter.open_regular(&name, libc::O_RDONLY, 0o600).is_err()
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn valid_authority_shape(authority: &AnchoredDirectory) -> Result<bool, HostFailure> {
    let names = authority.entry_names()?;
    if names.is_empty() {
        return Ok(true);
    }
    if names
        != AUTHORITY_ENTRIES
            .iter()
            .map(|value| (*value).to_owned())
            .collect()
    {
        return Ok(false);
    }
    for name in AUTHORITY_ENTRIES {
        if authority.open_regular(name, libc::O_RDONLY, 0o600).is_err() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn validate_legacy(adapter: &AnchoredDirectory) -> Result<(usize, usize, usize), ()> {
    let names = adapter.entry_names().map_err(|_| ())?;
    let singleton_count = usize::from(names.contains(CONTINUITY_CHECKPOINT_NAME));
    let continuation_count = match adapter.open_child(CONTINUITY_DIRECTORY_NAME) {
        Ok(directory) => {
            continuity::validate_continuation_directory(&directory).map_err(|_| ())?;
            directory.entry_names().map_err(|_| ())?.len()
        }
        Err(HostFailure::Unavailable) => 0,
        Err(_) => return Err(()),
    };
    let mut event_count = 0;
    for name in &names {
        if [
            LOCK_NAME,
            CONTINUITY_CHECKPOINT_NAME,
            CONTINUITY_DIRECTORY_NAME,
        ]
        .contains(&name.as_str())
        {
            continue;
        }
        host_state::validate_event_leaf_name(name).map_err(|_| ())?;
        adapter
            .open_regular(name, libc::O_RDONLY, 0o600)
            .map_err(|_| ())?;
        event_count += 1;
    }
    continuity::all_checkpoints(adapter).map_err(|_| ())?;
    host_state::require_adapter_entries(adapter).map_err(|_| ())?;
    Ok((singleton_count, continuation_count, event_count))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HistoryRelation {
    RedundantEquivalent,
    CanonicalStrictlySupersedes,
    SingletonStrictlySupersedes,
    ConflictingHistories,
}

struct ReconciledHistory {
    relation: HistoryRelation,
    reserved: Option<RoutineReservedRecoveryAssessment>,
    terminal_events: TerminalEventAssessment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalEventAssessment {
    Complete,
    MissingRequired,
    Conflicting,
}

impl TerminalEventAssessment {
    const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Conflicting, _) | (_, Self::Conflicting) => Self::Conflicting,
            (Self::MissingRequired, _) | (_, Self::MissingRequired) => Self::MissingRequired,
            (Self::Complete, Self::Complete) => Self::Complete,
        }
    }
}

impl HistoryRelation {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RedundantEquivalent => "redundant_equivalent",
            Self::CanonicalStrictlySupersedes => "canonical_strictly_supersedes",
            Self::SingletonStrictlySupersedes => "singleton_strictly_supersedes",
            Self::ConflictingHistories => "conflicting_histories_hold",
        }
    }
}

fn reconcile_mixed_history(
    authority: &AnchoredDirectory,
    adapter: &AnchoredDirectory,
    launch: &AnchoredDirectory,
    binding: CheckpointBinding<'_>,
) -> Result<ReconciledHistory, HostFailure> {
    validate_legacy(adapter).map_err(|_| HostFailure::Invalid)?;
    let checkpoints = continuity::all_checkpoints(adapter)?;
    let singleton = checkpoints.first().ok_or(HostFailure::Invalid)?;
    let mut reserved = None::<(&str, RoutineReservedRecoveryAssessment)>;
    for checkpoint in &checkpoints {
        if let Some(assessment) = authenticate_legacy_checkpoint(authority, launch, checkpoint)?
            && reserved_matches_relation_and_diagnosis(singleton, checkpoint, binding)
        {
            match reserved {
                Some((grant, current))
                    if grant != checkpoint.attempt_grant() || current != assessment =>
                {
                    return Err(HostFailure::Invalid);
                }
                Some(_) => {}
                None => reserved = Some((checkpoint.attempt_grant(), assessment)),
            }
        }
    }
    let terminal_events = validate_terminal_event_history(adapter, &checkpoints)?;
    let relation = classify_history_relation(singleton, &checkpoints[1..])?;
    Ok(ReconciledHistory {
        relation,
        // A stale `Reserved` assessment is projected only when the singleton
        // and canonical records are byte-equivalent duplicates. Ordered or
        // conflicting histories retain their existing relation-specific HOLD
        // instead of letting an older reservation obscure newer evidence.
        reserved: (relation == HistoryRelation::RedundantEquivalent)
            .then(|| reserved.map(|(_, assessment)| assessment))
            .flatten(),
        terminal_events,
    })
}

fn reserved_matches_relation_and_diagnosis(
    singleton: &ContinuationCheckpoint,
    checkpoint: &ContinuationCheckpoint,
    binding: CheckpointBinding<'_>,
) -> bool {
    same_binding(singleton, checkpoint) && checkpoint_matches_diagnosis(checkpoint, binding)
}

fn checkpoint_matches_diagnosis(
    checkpoint: &ContinuationCheckpoint,
    binding: CheckpointBinding<'_>,
) -> bool {
    // The private record authenticates the historical context/candidate/plan/
    // snapshot fields. Migration diagnosis must additionally confine that
    // historical attempt to the repository the caller is diagnosing; current
    // identities are expected to drift after a later binding advanced the
    // ledger.
    checkpoint.target() == binding.target().to_str().unwrap_or_default()
}

fn classify_history_relation(
    singleton: &ContinuationCheckpoint,
    canonical: &[ContinuationCheckpoint],
) -> Result<HistoryRelation, HostFailure> {
    if canonical.len() != 1 {
        return Ok(HistoryRelation::ConflictingHistories);
    }
    let matching = canonical
        .iter()
        .filter(|candidate| same_binding(singleton, candidate))
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return Ok(HistoryRelation::ConflictingHistories);
    }
    let canonical = matching[0];
    if serde_json::to_vec(singleton).map_err(|_| HostFailure::Invalid)?
        == serde_json::to_vec(canonical).map_err(|_| HostFailure::Invalid)?
    {
        return Ok(HistoryRelation::RedundantEquivalent);
    }
    if causally_supersedes(canonical, singleton) {
        return Ok(HistoryRelation::CanonicalStrictlySupersedes);
    }
    if causally_supersedes(singleton, canonical) {
        return Ok(HistoryRelation::SingletonStrictlySupersedes);
    }
    Ok(HistoryRelation::ConflictingHistories)
}

fn same_binding(left: &ContinuationCheckpoint, right: &ContinuationCheckpoint) -> bool {
    left.target() == right.target()
        && left.context_id() == right.context_id()
        && left.candidate_id() == right.candidate_id()
        && left.plan_id() == right.plan_id()
        && left.snapshot_id() == right.snapshot_id()
        && left.execution_id() == right.execution_id()
}

fn causally_supersedes(
    successor: &ContinuationCheckpoint,
    predecessor: &ContinuationCheckpoint,
) -> bool {
    successor.generation() > predecessor.generation()
        && successor
            .predecessor_continuations()
            .iter()
            .any(|value| value == predecessor.continuation())
}

fn authenticate_legacy_checkpoint(
    authority: &AnchoredDirectory,
    launch: &AnchoredDirectory,
    checkpoint: &ContinuationCheckpoint,
) -> Result<Option<RoutineReservedRecoveryAssessment>, HostFailure> {
    let authenticate = |allow_stale_head| {
        crate::routine_work::authenticate_public_routine_checkpoint(
            RoutineCustodyCapability::issue_from_host(
                super::super::HostCustodyIssuance::from_legacy_directories(authority, launch)?,
            ),
            Path::new(checkpoint.target()),
            checkpoint.context_id(),
            checkpoint.candidate_id(),
            checkpoint.plan_id(),
            checkpoint.snapshot_id(),
            checkpoint.continuation(),
            checkpoint.recovery_marker(),
            checkpoint.predecessor_continuations(),
            checkpoint.attempt_grant(),
            checkpoint.ledger_head(),
            checkpoint.state(),
            checkpoint
                .terminal_outcome()
                .map(|outcome| outcome.as_str()),
            allow_stale_head,
        )
        .map_err(|_| HostFailure::Invalid)
    };
    if authenticate(false).is_ok() {
        return Ok(None);
    }
    if checkpoint.state() == "reserved" && checkpoint.terminal_outcome().is_none() {
        return crate::routine_work::assess_public_routine_reserved_recovery(
            RoutineCustodyCapability::issue_from_host(
                super::super::HostCustodyIssuance::from_legacy_directories(authority, launch)?,
            ),
            Path::new(checkpoint.target()),
            checkpoint.context_id(),
            checkpoint.candidate_id(),
            checkpoint.plan_id(),
            checkpoint.snapshot_id(),
            checkpoint.continuation(),
            checkpoint.recovery_marker(),
            checkpoint.predecessor_continuations(),
            checkpoint.attempt_grant(),
            checkpoint.ledger_head(),
        )
        .map(Some)
        .map_err(|_| HostFailure::Invalid);
    }
    if !may_authenticate_stale_legacy_checkpoint(checkpoint) {
        return Err(HostFailure::Invalid);
    }
    authenticate(true).map(|()| None)
}

fn may_authenticate_stale_legacy_checkpoint(checkpoint: &ContinuationCheckpoint) -> bool {
    (checkpoint.state() == "reconciled" && checkpoint.terminal_outcome().is_none())
        || (checkpoint.is_terminal() && checkpoint.terminal_outcome().is_some())
}

fn validate_terminal_event_history(
    adapter: &AnchoredDirectory,
    checkpoints: &[ContinuationCheckpoint],
) -> Result<TerminalEventAssessment, HostFailure> {
    let mut expected = BTreeMap::<String, Vec<(crate::observability::SemanticEvent, bool)>>::new();
    for checkpoint in checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.is_terminal() && checkpoint.has_current_event_authority())
    {
        let target = Path::new(checkpoint.target());
        let leaf = host_state::event_leaf_name(
            target,
            checkpoint.context_id(),
            checkpoint.candidate_id(),
            "successor-runtime",
        )?;
        expected.entry(leaf).or_default().push((
            super::super::terminal_semantic_event_from_checkpoint(target, checkpoint)?,
            checkpoint.state() == "terminal-event-joined",
        ));
    }
    let actual_leaves = adapter
        .entry_names()?
        .into_iter()
        .filter(|name| name.starts_with(EVENT_FILE_PREFIX) && name.ends_with(EVENT_FILE_SUFFIX))
        .collect::<BTreeSet<_>>();
    let expected_leaves = expected.keys().cloned().collect::<BTreeSet<_>>();
    let mut assessment = classify_terminal_leaf_sets(&expected_leaves, &actual_leaves);
    for (leaf, expected_events) in expected {
        if !actual_leaves.contains(&leaf) {
            continue;
        }
        let binding = expected_events
            .first()
            .ok_or(HostFailure::Invalid)?
            .0
            .clone();
        let opened = adapter.open_regular(&leaf, libc::O_RDONLY, 0o600)?;
        let store = crate::observability::EventStore::open_descriptor_bound(
            adapter.file.try_clone().map_err(|_| HostFailure::Invalid)?,
            &leaf,
            &opened,
            adapter.identity.owner,
            0o600,
            binding.context_id(),
            binding.candidate_id(),
            binding.source_id(),
        )
        .map_err(|_| HostFailure::Invalid)?;
        let (events, duplicate_or_conflicting_event_id) = store
            .read_events_for_migration_admission()
            .map_err(|_| HostFailure::Invalid)?;
        if duplicate_or_conflicting_event_id {
            assessment = assessment.merge(TerminalEventAssessment::Conflicting);
        }
        assessment = assessment.merge(classify_terminal_event_rows(&expected_events, &events));
    }
    Ok(assessment)
}

fn classify_terminal_leaf_sets(
    expected: &BTreeSet<String>,
    actual: &BTreeSet<String>,
) -> TerminalEventAssessment {
    if actual.iter().any(|leaf| !expected.contains(leaf)) {
        TerminalEventAssessment::Conflicting
    } else if expected.iter().any(|leaf| !actual.contains(leaf)) {
        TerminalEventAssessment::MissingRequired
    } else {
        TerminalEventAssessment::Complete
    }
}

fn classify_terminal_event_rows(
    expected: &[(crate::observability::SemanticEvent, bool)],
    actual: &[crate::observability::SemanticEvent],
) -> TerminalEventAssessment {
    let duplicate_actual = actual
        .iter()
        .enumerate()
        .any(|(index, event)| actual[..index].iter().any(|prior| prior == event));
    let unexpected = actual
        .iter()
        .any(|event| !expected.iter().any(|(candidate, _)| candidate == event));
    // A byte-identical singleton/canonical pair legitimately contributes the
    // same expected semantic event twice. Treat those expectations as one;
    // duplicate rows in the observed journal remain conflicting evidence.
    if duplicate_actual || unexpected {
        return TerminalEventAssessment::Conflicting;
    }
    if expected
        .iter()
        .any(|(event, required)| *required && !actual.iter().any(|candidate| candidate == event))
    {
        TerminalEventAssessment::MissingRequired
    } else {
        TerminalEventAssessment::Complete
    }
}

fn counts(adapter: &AnchoredDirectory) -> Result<(usize, usize, usize), HostFailure> {
    let names = adapter.entry_names()?;
    let singleton = usize::from(names.contains(CONTINUITY_CHECKPOINT_NAME));
    let canonical = match adapter.open_child(CONTINUITY_DIRECTORY_NAME) {
        Ok(directory) => directory.entry_names()?.len(),
        Err(HostFailure::Unavailable) => 0,
        Err(error) => return Err(error),
    };
    let events = names
        .iter()
        .filter(|name| name.starts_with(EVENT_FILE_PREFIX) && name.ends_with(EVENT_FILE_SUFFIX))
        .count();
    Ok((singleton, canonical, events))
}

fn inventory(
    owner_parent: &AnchoredDirectory,
    state: &AnchoredDirectory,
    authority: &AnchoredDirectory,
    adapter: &AnchoredDirectory,
    launch: &AnchoredDirectory,
    observed_owner: &str,
    ignored_parent_entries: &[&str],
) -> Result<BTreeMap<String, InventoryEntry>, HostFailure> {
    let mut rows = BTreeMap::new();
    let mut parent_inventory = directory_inventory(owner_parent);
    // Parent `nlink` is not a stable identity component on the supported
    // filesystem and necessarily changes on some filesystems when the
    // transaction adds the fresh owner beside its quarantine. Exact parent
    // entry names and child identities remain inventoried below.
    parent_inventory.identity.links = 0;
    rows.insert("owner_parent".to_owned(), parent_inventory);
    rows.insert("state".to_owned(), directory_inventory(state));
    rows.insert("authority".to_owned(), directory_inventory(authority));
    rows.insert("adapter".to_owned(), directory_inventory(adapter));
    rows.insert("launch".to_owned(), directory_inventory(launch));
    capture_scoped_parent_entries(
        owner_parent,
        state,
        observed_owner,
        ignored_parent_entries,
        &mut rows,
    )?;
    capture_entries("authority", authority, &mut rows)?;
    capture_entries("adapter", adapter, &mut rows)?;
    capture_entries("launch", launch, &mut rows)?;
    if let Ok(directory) = adapter.open_child(CONTINUITY_DIRECTORY_NAME) {
        rows.insert(
            "adapter/continuations".to_owned(),
            directory_inventory(&directory),
        );
        capture_entries("adapter/continuations", &directory, &mut rows)?;
    }
    owner_parent.verify()?;
    state.verify()?;
    authority.verify()?;
    adapter.verify()?;
    launch.verify()?;
    Ok(rows)
}

fn capture_scoped_parent_entries(
    owner_parent: &AnchoredDirectory,
    state: &AnchoredDirectory,
    observed_owner: &str,
    ignored_parent_entries: &[&str],
    rows: &mut BTreeMap<String, InventoryEntry>,
) -> Result<(), HostFailure> {
    let ignored = ignored_parent_entries
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    if ignored.len() != ignored_parent_entries.len()
        || !valid_parent_transition_phase(observed_owner, &ignored)
    {
        return Err(HostFailure::Invalid);
    }
    let expected = ignored
        .iter()
        .cloned()
        .chain(std::iter::once(observed_owner.to_owned()))
        .collect::<BTreeSet<_>>();
    let discovered = scoped_reserved_parent_names(owner_parent, observed_owner)?;
    if discovered != expected {
        return Err(HostFailure::Invalid);
    }

    let mut identities = BTreeMap::new();
    for name in &discovered {
        let observed = owner_parent.stat(name)?.ok_or(HostFailure::Invalid)?;
        if observed.mode & libc::S_IFMT as u32 != libc::S_IFDIR as u32 {
            return Err(HostFailure::Invalid);
        }
        let opened = owner_parent.open_child(name)?;
        if opened.identity != observed
            || (name == observed_owner && opened.identity != state.identity)
        {
            return Err(HostFailure::Invalid);
        }
        opened.verify()?;
        identities.insert(name.clone(), observed);
    }
    if scoped_reserved_parent_names(owner_parent, observed_owner)? != discovered
        || identities
            .iter()
            .any(|(name, identity)| owner_parent.stat(name).ok().flatten() != Some(*identity))
    {
        return Err(HostFailure::Invalid);
    }

    for (name, identity) in identities {
        let path = if ignored.contains(&name) {
            format!("{IGNORED_PARENT_SNAPSHOT_PREFIX}{name}")
        } else {
            format!("owner_parent/{}", STATE_COMPONENTS[3])
        };
        if rows
            .insert(
                path,
                InventoryEntry {
                    identity,
                    content_sha256: None,
                },
            )
            .is_some()
        {
            return Err(HostFailure::Invalid);
        }
    }
    Ok(())
}

fn scoped_reserved_parent_names(
    owner_parent: &AnchoredDirectory,
    observed_owner: &str,
) -> Result<BTreeSet<String>, HostFailure> {
    Ok(owner_parent
        .entry_names()?
        .into_iter()
        .filter(|name| {
            name == observed_owner
                || name == STATE_COMPONENTS[3]
                || name == BOOTSTRAP_STAGE
                || name.starts_with("routine-public.quarantine-")
        })
        .collect())
}

fn valid_parent_transition_phase(observed_owner: &str, ignored: &BTreeSet<String>) -> bool {
    if observed_owner == STATE_COMPONENTS[3] {
        return ignored.is_empty() || ignored == &BTreeSet::from([BOOTSTRAP_STAGE.to_owned()]);
    }
    if observed_owner == BOOTSTRAP_STAGE || observed_owner.starts_with("routine-public.quarantine-")
    {
        return ignored == &BTreeSet::from([STATE_COMPONENTS[3].to_owned()]);
    }
    false
}

fn quarantine_plan(
    inventory: &BTreeMap<String, InventoryEntry>,
    history_relation: &'static str,
    target: &Path,
) -> Result<RoutineStateQuarantinePlan, HostFailure> {
    quarantine_plan_with_history_basis(
        inventory,
        history_relation,
        target,
        QuarantinePlanHistoryBasis::Authenticated,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuarantinePlanHistoryBasis {
    Authenticated,
    StaleReservedHistoricalHeadUnproven,
    ExplicitOperatorAbandonment,
}

impl QuarantinePlanHistoryBasis {
    fn authoritative_history(self, history_relation: &str) -> Result<&'static str, HostFailure> {
        Ok(match (self, history_relation) {
            (Self::Authenticated, "single_history_only") => "the_only_authenticated_history",
            (Self::Authenticated, "redundant_equivalent" | "canonical_strictly_supersedes") => {
                "canonical_authenticated_history"
            }
            (Self::Authenticated, "singleton_strictly_supersedes") => {
                "singleton_authenticated_history"
            }
            (Self::StaleReservedHistoricalHeadUnproven, "redundant_equivalent") => {
                "none_noncurrent_value_only_historical_head_unproven"
            }
            (
                Self::ExplicitOperatorAbandonment,
                "terminal_event_history_incomplete"
                | "terminal_event_history_conflicting"
                | "conflicting_histories_hold",
            ) => "none_explicit_operator_abandonment",
            _ => return Err(HostFailure::Invalid),
        })
    }

    fn bind_plan_identity(self, plan: &mut Sha256) {
        match self {
            Self::Authenticated => {}
            Self::StaleReservedHistoricalHeadUnproven => {
                plan.update(b"\0stale-reserved-head-evidence:");
                plan.update(STALE_RESERVED_HEAD_CEILING.as_bytes());
            }
            Self::ExplicitOperatorAbandonment => {
                plan.update(ABANDONMENT_CONSEQUENCE_BINDING);
            }
        }
    }

    const fn schema(self) -> &'static str {
        match self {
            Self::Authenticated | Self::StaleReservedHistoricalHeadUnproven => {
                QUARANTINE_PLAN_SCHEMA
            }
            Self::ExplicitOperatorAbandonment => ABANDONMENT_PLAN_SCHEMA,
        }
    }

    const fn identity_domain(self) -> &'static [u8] {
        match self {
            Self::Authenticated | Self::StaleReservedHistoricalHeadUnproven => {
                b"routine-state-quarantine-plan-v5\0"
            }
            Self::ExplicitOperatorAbandonment => b"routine-state-quarantine-plan-v6\0",
        }
    }
}

fn quarantine_plan_with_history_basis(
    inventory: &BTreeMap<String, InventoryEntry>,
    history_relation: &'static str,
    target: &Path,
    history_basis: QuarantinePlanHistoryBasis,
) -> Result<RoutineStateQuarantinePlan, HostFailure> {
    let authoritative_history = history_basis.authoritative_history(history_relation)?;
    let source_inventory_sha256 = inventory_sha256(inventory);
    let source_owner_tree_sha256 = owner_tree_sha256(inventory);
    let target_path_sha256 = target_path_sha256(target);
    let plan_id = quarantine_plan_id(
        &source_inventory_sha256,
        &source_owner_tree_sha256,
        &target_path_sha256,
        history_relation,
        history_basis,
    );
    let suffix = plan_id
        .strip_prefix("routine-quarantine-sha256:")
        .ok_or(HostFailure::Invalid)?
        .to_owned();
    Ok(RoutineStateQuarantinePlan {
        schema_version: history_basis.schema().to_owned(),
        plan_id,
        source_inventory_sha256,
        source_owner_tree_sha256,
        target_path_sha256,
        history_relation: history_relation.to_owned(),
        authoritative_history: authoritative_history.to_owned(),
        source_owner: STATE_COMPONENTS[3].to_owned(),
        quarantine_owner: format!("{}.quarantine-{}", STATE_COMPONENTS[3], suffix),
        target_format: "routine-host-state-v8".to_owned(),
        strategy: QUARANTINE_STRATEGY.to_owned(),
        apply_capability: QUARANTINE_APPLY_CAPABILITY.to_owned(),
        operations: QUARANTINE_PLAN_OPERATIONS
            .iter()
            .map(|operation| (*operation).to_owned())
            .collect(),
        rollback: QUARANTINE_ROLLBACK.to_owned(),
    })
}

fn quarantine_plan_id(
    source_inventory_sha256: &str,
    source_owner_tree_sha256: &str,
    target_path_sha256: &str,
    history_relation: &str,
    history_basis: QuarantinePlanHistoryBasis,
) -> String {
    let mut plan = Sha256::new();
    plan.update(history_basis.identity_domain());
    plan.update(source_inventory_sha256.as_bytes());
    plan.update([0]);
    plan.update(source_owner_tree_sha256.as_bytes());
    plan.update([0]);
    plan.update(target_path_sha256.as_bytes());
    plan.update([0]);
    plan.update(history_relation.as_bytes());
    history_basis.bind_plan_identity(&mut plan);
    plan.update(QUARANTINE_PLAN_STRATEGY_BINDING);
    format!("routine-quarantine-sha256:{:x}", plan.finalize())
}

pub(super) fn quarantine_plan_is_semantically_valid(plan: &RoutineStateQuarantinePlan) -> bool {
    let history_basis = match (
        plan.history_relation.as_str(),
        plan.authoritative_history.as_str(),
    ) {
        ("single_history_only", "the_only_authenticated_history")
        | ("redundant_equivalent", "canonical_authenticated_history")
        | ("canonical_strictly_supersedes", "canonical_authenticated_history")
        | ("singleton_strictly_supersedes", "singleton_authenticated_history") => {
            QuarantinePlanHistoryBasis::Authenticated
        }
        ("redundant_equivalent", "none_noncurrent_value_only_historical_head_unproven") => {
            QuarantinePlanHistoryBasis::StaleReservedHistoricalHeadUnproven
        }
        (
            "terminal_event_history_incomplete"
            | "terminal_event_history_conflicting"
            | "conflicting_histories_hold",
            "none_explicit_operator_abandonment",
        ) => QuarantinePlanHistoryBasis::ExplicitOperatorAbandonment,
        _ => return false,
    };
    let expected_plan_id = quarantine_plan_id(
        &plan.source_inventory_sha256,
        &plan.source_owner_tree_sha256,
        &plan.target_path_sha256,
        &plan.history_relation,
        history_basis,
    );
    let Some(suffix) = expected_plan_id.strip_prefix("routine-quarantine-sha256:") else {
        return false;
    };
    plan.schema_version == history_basis.schema()
        && plan.plan_id == expected_plan_id
        && is_sha256(&plan.source_inventory_sha256)
        && is_sha256(&plan.source_owner_tree_sha256)
        && is_sha256(&plan.target_path_sha256)
        && plan.source_owner == STATE_COMPONENTS[3]
        && plan.quarantine_owner == format!("{}.quarantine-{suffix}", STATE_COMPONENTS[3])
        && plan.target_format == "routine-host-state-v8"
        && plan.strategy == QUARANTINE_STRATEGY
        && plan.apply_capability == QUARANTINE_APPLY_CAPABILITY
        && plan
            .operations
            .iter()
            .map(String::as_str)
            .eq(QUARANTINE_PLAN_OPERATIONS.iter().copied())
        && plan.rollback == QUARANTINE_ROLLBACK
}

fn is_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn owner_tree_sha256(inventory: &BTreeMap<String, InventoryEntry>) -> String {
    inventory_sha256_entries(
        inventory.iter().filter(|(path, _)| {
            path.as_str() != "owner_parent"
                && !path.starts_with("owner_parent/")
                && !path.starts_with(IGNORED_PARENT_SNAPSHOT_PREFIX)
        }),
        b"routine-state-quarantine-owner-tree-v2\0",
    )
}

pub(super) fn observed_owner_tree_sha256(
    owner_parent: &AnchoredDirectory,
    state: &AnchoredDirectory,
    observed_owner: &str,
    ignored_parent_entries: &[&str],
) -> Result<String, HostFailure> {
    let authority = state.open_child(AUTHORITY_DIRECTORY)?;
    let adapter = state.open_child(ADAPTER_DIRECTORY)?;
    let launch = state.open_child(LAUNCH_DIRECTORY)?;
    let inventory = inventory(
        owner_parent,
        state,
        &authority,
        &adapter,
        &launch,
        observed_owner,
        ignored_parent_entries,
    )?;
    Ok(owner_tree_sha256(&inventory))
}

pub(super) fn observed_inventory_digests(
    owner_parent: &AnchoredDirectory,
    state: &AnchoredDirectory,
    observed_owner: &str,
    ignored_parent_entries: &[&str],
) -> Result<(String, String), HostFailure> {
    let authority = state.open_child(AUTHORITY_DIRECTORY)?;
    let adapter = state.open_child(ADAPTER_DIRECTORY)?;
    let launch = state.open_child(LAUNCH_DIRECTORY)?;
    let inventory = inventory(
        owner_parent,
        state,
        &authority,
        &adapter,
        &launch,
        observed_owner,
        ignored_parent_entries,
    )?;
    Ok((inventory_sha256(&inventory), owner_tree_sha256(&inventory)))
}

fn target_path_sha256(target: &Path) -> String {
    let mut digest = Sha256::new();
    digest.update(b"routine-state-quarantine-target-path-v1\0");
    digest.update((target.as_os_str().as_bytes().len() as u64).to_be_bytes());
    digest.update(target.as_os_str().as_bytes());
    format!("sha256:{:x}", digest.finalize())
}

fn inventory_sha256(inventory: &BTreeMap<String, InventoryEntry>) -> String {
    inventory_sha256_entries(
        inventory
            .iter()
            .filter(|(path, _)| !path.starts_with(IGNORED_PARENT_SNAPSHOT_PREFIX)),
        b"routine-state-quarantine-source-inventory-v3\0",
    )
}

fn inventory_sha256_entries<'a>(
    inventory: impl Iterator<Item = (&'a String, &'a InventoryEntry)>,
    domain: &[u8],
) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    for (path, identity) in inventory {
        digest.update(path.as_bytes());
        digest.update([0]);
        // `st_dev` is a mount-session observation on macOS, not durable object
        // identity. Descriptor/path comparisons retain the device number while
        // an operation is open; persisted migration plans bind only the stable
        // inventory dimensions so an exact crash replay survives a reboot.
        digest.update(identity.identity.inode.to_le_bytes());
        digest.update(identity.identity.owner.to_le_bytes());
        digest.update(identity.identity.mode.to_le_bytes());
        digest.update(identity.identity.links.to_le_bytes());
        match identity.content_sha256 {
            Some(content) => {
                digest.update([1]);
                digest.update(content);
            }
            None => digest.update([0]),
        }
    }
    format!("sha256:{:x}", digest.finalize())
}

fn capture_entries(
    prefix: &str,
    directory: &AnchoredDirectory,
    rows: &mut BTreeMap<String, InventoryEntry>,
) -> Result<(), HostFailure> {
    for name in directory.entry_names()? {
        let identity = directory.stat(&name)?.ok_or(HostFailure::Invalid)?;
        let content_sha256 = if identity.mode & libc::S_IFMT as u32 == libc::S_IFREG as u32 {
            Some(content_sha256(
                directory,
                &name,
                identity,
                inventory_file_limit(prefix, &name)?,
            )?)
        } else if identity.mode & libc::S_IFMT as u32 == libc::S_IFDIR as u32 {
            None
        } else {
            return Err(HostFailure::Invalid);
        };
        rows.insert(
            format!("{prefix}/{name}"),
            InventoryEntry {
                identity,
                content_sha256,
            },
        );
    }
    Ok(())
}

fn directory_inventory(directory: &AnchoredDirectory) -> InventoryEntry {
    InventoryEntry {
        identity: directory.identity,
        content_sha256: None,
    }
}

fn inventory_file_limit(prefix: &str, name: &str) -> Result<u64, HostFailure> {
    match prefix {
        "authority" => Ok(MAX_AUTHORITY_ENTRY_BYTES),
        "adapter/continuations" => Ok(MAX_CHECKPOINT_ENTRY_BYTES),
        "adapter" if name == LOCK_NAME => Ok(MAX_LOCK_ENTRY_BYTES),
        "adapter" if name == CONTINUITY_CHECKPOINT_NAME => Ok(MAX_CHECKPOINT_ENTRY_BYTES),
        "adapter" if name.starts_with(EVENT_FILE_PREFIX) && name.ends_with(EVENT_FILE_SUFFIX) => {
            Ok(MAX_EVENT_ENTRY_BYTES)
        }
        _ => Err(HostFailure::Invalid),
    }
}

fn content_sha256(
    directory: &AnchoredDirectory,
    name: &str,
    expected: Identity,
    limit: u64,
) -> Result<[u8; 32], HostFailure> {
    let file = directory.open_regular(name, libc::O_RDONLY, 0o600)?;
    if identity(&file.metadata().map_err(|_| HostFailure::Invalid)?) != expected {
        return Err(HostFailure::Invalid);
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| HostFailure::Invalid)?;
    if bytes.len() as u64 > limit || directory.stat(name)? != Some(expected) {
        return Err(HostFailure::Invalid);
    }
    Ok(Sha256::digest(&bytes).into())
}

fn reconciled_hold(
    singleton: usize,
    canonical: usize,
    events: usize,
    history: ReconciledHistory,
    inventory: &BTreeMap<String, InventoryEntry>,
    target: &Path,
    mode: LegacyAdmissionMode,
) -> Result<RoutineStateMigrationAdmission, HostFailure> {
    let relation = history.relation;
    if relation == HistoryRelation::RedundantEquivalent
        && let Some(assessment) = history.reserved
    {
        let (reserved_recovery, reserved_next_action) =
            reserved_recovery_admission::admission(assessment);
        let quarantine_plan = if canonical == 1
            && raw_duplicate_history(inventory)
            && assessment
                == (RoutineReservedRecoveryAssessment {
                    owner: ReservedRecoveryOwnerObservation::NotObserved,
                    effect: ReservedRecoveryEffectEvidence::PristineNoEffect,
                }) {
            Some(quarantine_plan_with_history_basis(
                inventory,
                relation.as_str(),
                target,
                QuarantinePlanHistoryBasis::StaleReservedHistoricalHeadUnproven,
            )?)
        } else {
            None
        };
        return Ok(RoutineStateMigrationAdmission {
            status: reserved_recovery.status,
            format_status: "absent_legacy",
            legacy_singleton_count: singleton,
            canonical_continuation_count: canonical,
            event_journal_count: events,
            history_relation: relation.as_str(),
            next_action: if quarantine_plan.is_some() {
                QUARANTINE_PLAN_NEXT_ACTION
            } else {
                reserved_next_action
            },
            reserved_recovery: Some(reserved_recovery),
            quarantine_plan,
        });
    }
    let quarantine_plan = match relation {
        HistoryRelation::RedundantEquivalent
        | HistoryRelation::CanonicalStrictlySupersedes
        | HistoryRelation::SingletonStrictlySupersedes => {
            Some(quarantine_plan(inventory, relation.as_str(), target)?)
        }
        HistoryRelation::ConflictingHistories
            if mode == LegacyAdmissionMode::ExplicitAbandonment =>
        {
            Some(quarantine_plan_with_history_basis(
                inventory,
                relation.as_str(),
                target,
                QuarantinePlanHistoryBasis::ExplicitOperatorAbandonment,
            )?)
        }
        HistoryRelation::ConflictingHistories => None,
    };
    Ok(RoutineStateMigrationAdmission {
        status: match relation {
            HistoryRelation::RedundantEquivalent
            | HistoryRelation::CanonicalStrictlySupersedes
            | HistoryRelation::SingletonStrictlySupersedes => {
                "history_relation_established_preserve_and_hold"
            }
            HistoryRelation::ConflictingHistories => "conflicting_histories_preserve_and_hold",
        },
        format_status: "absent_legacy",
        legacy_singleton_count: singleton,
        canonical_continuation_count: canonical,
        event_journal_count: events,
        history_relation: relation.as_str(),
        next_action: if relation == HistoryRelation::ConflictingHistories
            && quarantine_plan.is_some()
        {
            ABANDONMENT_PLAN_NEXT_ACTION
        } else if quarantine_plan.is_some() {
            QUARANTINE_PLAN_NEXT_ACTION
        } else {
            "preserve all bytes; resolve the conflicting authenticated histories without proposing a migration"
        },
        reserved_recovery: None,
        quarantine_plan,
    })
}

fn terminal_event_abandonment_admission(
    singleton: usize,
    canonical: usize,
    events: usize,
    assessment: TerminalEventAssessment,
    inventory: &BTreeMap<String, InventoryEntry>,
    target: &Path,
) -> Result<RoutineStateMigrationAdmission, HostFailure> {
    let history_relation = match assessment {
        TerminalEventAssessment::MissingRequired => "terminal_event_history_incomplete",
        TerminalEventAssessment::Conflicting => "terminal_event_history_conflicting",
        TerminalEventAssessment::Complete => return Err(HostFailure::Invalid),
    };
    Ok(RoutineStateMigrationAdmission {
        status: "legacy_history_abandonment_requires_explicit_retirement_approval",
        format_status: "absent_legacy",
        legacy_singleton_count: singleton,
        canonical_continuation_count: canonical,
        event_journal_count: events,
        history_relation,
        next_action: ABANDONMENT_PLAN_NEXT_ACTION,
        reserved_recovery: None,
        quarantine_plan: Some(quarantine_plan_with_history_basis(
            inventory,
            history_relation,
            target,
            QuarantinePlanHistoryBasis::ExplicitOperatorAbandonment,
        )?),
    })
}

fn raw_duplicate_history(inventory: &BTreeMap<String, InventoryEntry>) -> bool {
    let Some(singleton) = inventory
        .get(&format!("adapter/{CONTINUITY_CHECKPOINT_NAME}"))
        .and_then(|entry| entry.content_sha256)
    else {
        return false;
    };
    let canonical = inventory
        .iter()
        .filter_map(|(path, entry)| {
            (path.starts_with("adapter/continuations/routine-continuation-")
                && path.ends_with(".json"))
            .then_some(entry.content_sha256)
            .flatten()
        })
        .collect::<Vec<_>>();
    canonical.len() == 1 && canonical[0] == singleton
}

fn terminal_event_hold(
    singleton: usize,
    canonical: usize,
    events: usize,
    assessment: TerminalEventAssessment,
) -> RoutineStateMigrationAdmission {
    let (status, history_relation, next_action) = match assessment {
        TerminalEventAssessment::MissingRequired => (
            "terminal_event_history_incomplete_preserve_and_hold",
            "terminal_event_history_incomplete",
            "preserve all bytes; inspect the missing authenticated terminal-event evidence without synthesizing, deleting, or rewriting history; retry diagnosis only after the exact evidence is lawfully restored",
        ),
        TerminalEventAssessment::Conflicting => (
            "terminal_event_history_conflicting_preserve_and_hold",
            "terminal_event_history_conflicting",
            "preserve all bytes; inspect the conflicting terminal-event evidence without synthesizing, deleting, or rewriting history; do not propose or apply a migration",
        ),
        TerminalEventAssessment::Complete => {
            return unsafe_hold("absent");
        }
    };
    RoutineStateMigrationAdmission {
        status,
        format_status: "absent_legacy",
        legacy_singleton_count: singleton,
        canonical_continuation_count: canonical,
        event_journal_count: events,
        history_relation,
        next_action,
        reserved_recovery: None,
        quarantine_plan: None,
    }
}

fn stale_writer_hold(format_status: &'static str) -> RoutineStateMigrationAdmission {
    RoutineStateMigrationAdmission {
        status: "stale_writer_preserve_and_retry",
        format_status,
        legacy_singleton_count: 0,
        canonical_continuation_count: 0,
        event_journal_count: 0,
        history_relation: "stale_writer_retry",
        next_action: "preserve all bytes and retry diagnosis after the current writer releases the store lock",
        reserved_recovery: None,
        quarantine_plan: None,
    }
}

fn unsafe_hold(format_status: &'static str) -> RoutineStateMigrationAdmission {
    RoutineStateMigrationAdmission {
        status: "unsafe_layout_preserve_and_hold",
        format_status,
        legacy_singleton_count: 0,
        canonical_continuation_count: 0,
        event_journal_count: 0,
        history_relation: "unsafe_or_unauthenticated",
        next_action: "preserve all bytes and inspect the unsupported or unsafe entry through a confined reader",
        reserved_recovery: None,
        quarantine_plan: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn checkpoint(
        generation: u64,
        continuation: &str,
        predecessors: &[&str],
    ) -> ContinuationCheckpoint {
        serde_json::from_value(serde_json::json!({
            "schema_version": "RoutineContinuationCheckpoint-v7",
            "generation": generation,
            "target": "/private/tmp/target",
            "context_id": "sha256:context",
            "candidate_id": "sha256:candidate",
            "plan_id": "sha256:plan",
            "snapshot_id": "sha256:snapshot",
            "execution_id": "sha256:execution",
            "continuation": continuation,
            "predecessor_continuations": predecessors,
            "recovery_marker": "sha256:recovery",
            "attempt_grant": "sha256:grant",
            "authenticated_ledger_head": "sha256:head",
            "finding_binding": null,
            "operation": "terminal",
            "terminal_outcome": "complete",
            "state": "terminal-event-joined",
            "event_id": "routine-terminal-event",
            "event_observed_at_unix_ms": 1,
            "event_sequence": 1,
            "event_parent_id": null,
            "event_status": "complete",
            "event_transition": "execute-to-complete"
        }))
        .unwrap()
    }

    fn raw_duplicate_inventory() -> BTreeMap<String, InventoryEntry> {
        let identity = Identity {
            device: 1,
            inode: 2,
            owner: 501,
            mode: 0o100600,
            links: 1,
        };
        let digest: [u8; 32] = Sha256::digest(b"same raw checkpoint").into();
        BTreeMap::from([
            (
                format!("adapter/{CONTINUITY_CHECKPOINT_NAME}"),
                InventoryEntry {
                    identity,
                    content_sha256: Some(digest),
                },
            ),
            (
                "adapter/continuations/routine-continuation-a.json".to_owned(),
                InventoryEntry {
                    identity,
                    content_sha256: Some(digest),
                },
            ),
        ])
    }

    fn create_empty_legacy_owner(root: &Path) {
        let state = root.join(STATE_COMPONENTS[3]);
        for directory in [
            root.to_path_buf(),
            state.clone(),
            state.join(AUTHORITY_DIRECTORY),
            state.join(ADAPTER_DIRECTORY),
            state.join(LAUNCH_DIRECTORY),
        ] {
            fs::create_dir(directory.clone()).unwrap();
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }

    fn test_inventory(
        root: &Path,
        observed_owner: &str,
        ignored_parent_entries: &[&str],
    ) -> Result<BTreeMap<String, InventoryEntry>, HostFailure> {
        let state_path = root.join(observed_owner);
        let owner_parent =
            AnchoredDirectory::open_absolute(root, DirectorySecurity::PrivateAuthority)?;
        let state =
            AnchoredDirectory::open_absolute(&state_path, DirectorySecurity::PrivateAuthority)?;
        let authority = state.open_child(AUTHORITY_DIRECTORY)?;
        let adapter = state.open_child(ADAPTER_DIRECTORY)?;
        let launch = state.open_child(LAUNCH_DIRECTORY)?;
        inventory(
            &owner_parent,
            &state,
            &authority,
            &adapter,
            &launch,
            observed_owner,
            ignored_parent_entries,
        )
    }

    #[test]
    fn causal_relation_requires_a_strict_generation_and_explicit_predecessor() {
        let prior = checkpoint(1, "routine-cont-sha256:prior", &[]);
        let successor = checkpoint(
            2,
            "routine-cont-sha256:successor",
            &["routine-cont-sha256:prior"],
        );
        assert!(causally_supersedes(&successor, &prior));
        assert!(!causally_supersedes(&prior, &successor));
        assert!(!causally_supersedes(
            &checkpoint(2, "routine-cont-sha256:other", &[]),
            &prior
        ));
        assert!(!causally_supersedes(
            &checkpoint(
                1,
                "routine-cont-sha256:other",
                &["routine-cont-sha256:prior"]
            ),
            &prior
        ));
    }

    #[test]
    fn diagnosis_binding_rejects_target_relabel_but_allows_historical_ids() {
        let mut checkpoint = checkpoint(1, "routine-cont-sha256:binding", &[]);
        let binding = CheckpointBinding::new(
            Path::new("/private/tmp/target"),
            "sha256:context",
            "sha256:candidate",
            "sha256:plan",
            "sha256:snapshot",
            "sha256:execution",
        );
        assert!(checkpoint_matches_diagnosis(&checkpoint, binding));

        let mut relabelled = serde_json::to_value(&checkpoint).unwrap();
        relabelled["target"] = serde_json::Value::from("/private/tmp/relabelled");
        checkpoint = serde_json::from_value(relabelled).unwrap();
        assert!(!checkpoint_matches_diagnosis(&checkpoint, binding));
        let mut historical = serde_json::to_value(&checkpoint).unwrap();
        historical["target"] = serde_json::Value::from("/private/tmp/target");
        historical["context_id"] = serde_json::Value::from("sha256:historical-context");
        checkpoint = serde_json::from_value(historical).unwrap();
        assert!(checkpoint_matches_diagnosis(&checkpoint, binding));
    }

    #[test]
    fn reserved_projection_cannot_borrow_an_unrelated_duplicate_relation() {
        let singleton = checkpoint(1, "routine-cont-sha256:singleton", &[]);
        let mut unrelated =
            serde_json::to_value(checkpoint(1, "routine-cont-sha256:unrelated", &[])).unwrap();
        unrelated["target"] = serde_json::Value::from("/private/tmp/other-target");
        unrelated["context_id"] = serde_json::Value::from("sha256:other-context");
        let unrelated: ContinuationCheckpoint = serde_json::from_value(unrelated).unwrap();
        let other_binding = CheckpointBinding::new(
            Path::new("/private/tmp/other-target"),
            "sha256:other-context",
            "sha256:candidate",
            "sha256:plan",
            "sha256:snapshot",
            "sha256:execution",
        );

        assert!(checkpoint_matches_diagnosis(&unrelated, other_binding));
        assert!(!reserved_matches_relation_and_diagnosis(
            &singleton,
            &unrelated,
            other_binding,
        ));
    }

    #[test]
    fn reserved_projection_requires_one_raw_identical_duplicate() {
        let mut inventory = raw_duplicate_inventory();
        assert!(raw_duplicate_history(&inventory));

        inventory
            .get_mut("adapter/continuations/routine-continuation-a.json")
            .unwrap()
            .content_sha256 = Some(Sha256::digest(b"semantic match, raw difference").into());
        assert!(!raw_duplicate_history(&inventory));

        let raw_different = reconciled_hold(
            1,
            1,
            0,
            ReconciledHistory {
                relation: HistoryRelation::RedundantEquivalent,
                reserved: Some(RoutineReservedRecoveryAssessment {
                    owner: ReservedRecoveryOwnerObservation::NotObserved,
                    effect: ReservedRecoveryEffectEvidence::PristineNoEffect,
                }),
                terminal_events: TerminalEventAssessment::Complete,
            },
            &inventory,
            Path::new("/private/tmp/target"),
            LegacyAdmissionMode::Migration,
        )
        .unwrap();
        assert!(raw_different.reserved_recovery.is_some());
        assert!(raw_different.quarantine_plan.is_none());
        assert!(!raw_different.next_action.contains("migrate apply"));

        let digest: [u8; 32] = Sha256::digest(b"same raw checkpoint").into();
        inventory
            .get_mut("adapter/continuations/routine-continuation-a.json")
            .unwrap()
            .content_sha256 = Some(digest);
        let identity = inventory
            .get(&format!("adapter/{CONTINUITY_CHECKPOINT_NAME}"))
            .unwrap()
            .identity;
        inventory.insert(
            "adapter/continuations/routine-continuation-b.json".to_owned(),
            InventoryEntry {
                identity,
                content_sha256: Some(digest),
            },
        );
        assert!(!raw_duplicate_history(&inventory));

        inventory
            .get_mut("adapter/continuations/routine-continuation-b.json")
            .unwrap()
            .content_sha256 = Some(Sha256::digest(b"valid disjoint canonical history").into());
        assert!(!raw_duplicate_history(&inventory));

        let held = reconciled_hold(
            1,
            2,
            0,
            ReconciledHistory {
                relation: HistoryRelation::RedundantEquivalent,
                reserved: Some(RoutineReservedRecoveryAssessment {
                    owner: ReservedRecoveryOwnerObservation::NotObserved,
                    effect: ReservedRecoveryEffectEvidence::PristineNoEffect,
                }),
                terminal_events: TerminalEventAssessment::Complete,
            },
            &inventory,
            Path::new("/private/tmp/target"),
            LegacyAdmissionMode::Migration,
        )
        .unwrap();
        assert!(held.reserved_recovery.is_some());
        assert!(held.quarantine_plan.is_none());
    }

    #[test]
    fn exact_abandoned_pristine_reserved_duplicate_mints_one_deterministic_ceiling_bound_plan() {
        let inventory = raw_duplicate_inventory();
        let target = Path::new("/private/tmp/target");
        let history = || ReconciledHistory {
            relation: HistoryRelation::RedundantEquivalent,
            reserved: Some(RoutineReservedRecoveryAssessment {
                owner: ReservedRecoveryOwnerObservation::NotObserved,
                effect: ReservedRecoveryEffectEvidence::PristineNoEffect,
            }),
            terminal_events: TerminalEventAssessment::Complete,
        };

        let first = reconciled_hold(
            1,
            1,
            0,
            history(),
            &inventory,
            target,
            LegacyAdmissionMode::Migration,
        )
        .unwrap();
        let second = reconciled_hold(
            1,
            1,
            0,
            history(),
            &inventory,
            target,
            LegacyAdmissionMode::Migration,
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.status,
            "stale_reserved_abandoned_candidate_preserve_and_hold"
        );
        assert_eq!(first.history_relation, "redundant_equivalent");
        assert!(first.reserved_recovery.is_some());
        let plan = first.quarantine_plan.as_ref().unwrap();
        assert_eq!(plan.schema_version, "RoutineStateQuarantinePlan-v5");
        assert_eq!(
            plan.authoritative_history,
            "none_noncurrent_value_only_historical_head_unproven"
        );
        assert_eq!(plan.apply_capability, "available_exact_record_only");
        assert!(
            first
                .next_action
                .contains("persist the exact ultragoal --json diagnose JSON")
        );
        assert!(first.next_action.contains(
            "ultragoal --json migrate apply --plan <relative-diagnosis-record> --accept-plan <quarantine_plan.plan_id>"
        ));

        let ordinary = quarantine_plan(&inventory, "redundant_equivalent", target).unwrap();
        assert_ne!(plan.plan_id, ordinary.plan_id);
        assert_ne!(plan.authoritative_history, ordinary.authoritative_history);

        let other_target = reconciled_hold(
            1,
            1,
            0,
            history(),
            &inventory,
            Path::new("/private/tmp/other-target"),
            LegacyAdmissionMode::Migration,
        )
        .unwrap();
        assert_ne!(plan.plan_id, other_target.quarantine_plan.unwrap().plan_id);
    }

    #[test]
    fn every_other_reserved_assessment_remains_planless() {
        let inventory = raw_duplicate_inventory();
        let target = Path::new("/private/tmp/target");
        let cases = [
            (
                ReservedRecoveryOwnerObservation::Active,
                ReservedRecoveryEffectEvidence::PristineNoEffect,
            ),
            (
                ReservedRecoveryOwnerObservation::Unavailable,
                ReservedRecoveryEffectEvidence::PristineNoEffect,
            ),
            (
                ReservedRecoveryOwnerObservation::Active,
                ReservedRecoveryEffectEvidence::EffectOrAmbiguityPresent,
            ),
            (
                ReservedRecoveryOwnerObservation::NotObserved,
                ReservedRecoveryEffectEvidence::EffectOrAmbiguityPresent,
            ),
            (
                ReservedRecoveryOwnerObservation::Unavailable,
                ReservedRecoveryEffectEvidence::EffectOrAmbiguityPresent,
            ),
        ];

        for (owner, effect) in cases {
            let admission = reconciled_hold(
                1,
                1,
                0,
                ReconciledHistory {
                    relation: HistoryRelation::RedundantEquivalent,
                    reserved: Some(RoutineReservedRecoveryAssessment { owner, effect }),
                    terminal_events: TerminalEventAssessment::Complete,
                },
                &inventory,
                target,
                LegacyAdmissionMode::Migration,
            )
            .unwrap();
            assert!(
                admission.reserved_recovery.is_some(),
                "{owner:?}/{effect:?}"
            );
            assert!(admission.quarantine_plan.is_none(), "{owner:?}/{effect:?}");
            assert!(
                !admission.next_action.contains("migrate apply"),
                "{owner:?}/{effect:?}"
            );
        }
    }

    #[test]
    fn closed_relation_classifier_distinguishes_equivalence_order_and_conflict() {
        let singleton = checkpoint(1, "routine-cont-sha256:singleton", &[]);
        assert_eq!(
            classify_history_relation(&singleton, std::slice::from_ref(&singleton)).unwrap(),
            HistoryRelation::RedundantEquivalent
        );

        let canonical = checkpoint(
            2,
            "routine-cont-sha256:canonical",
            &["routine-cont-sha256:singleton"],
        );
        assert_eq!(
            classify_history_relation(&singleton, std::slice::from_ref(&canonical)).unwrap(),
            HistoryRelation::CanonicalStrictlySupersedes
        );
        assert_eq!(
            classify_history_relation(&canonical, std::slice::from_ref(&singleton)).unwrap(),
            HistoryRelation::SingletonStrictlySupersedes
        );

        let unordered = checkpoint(2, "routine-cont-sha256:unordered", &[]);
        assert_eq!(
            classify_history_relation(&singleton, std::slice::from_ref(&unordered)).unwrap(),
            HistoryRelation::ConflictingHistories
        );
        let mut disjoint =
            serde_json::to_value(checkpoint(2, "routine-cont-sha256:disjoint", &[])).unwrap();
        disjoint["candidate_id"] = serde_json::json!("sha256:other-candidate");
        let disjoint: ContinuationCheckpoint = serde_json::from_value(disjoint).unwrap();
        assert_eq!(
            classify_history_relation(&singleton, std::slice::from_ref(&disjoint)).unwrap(),
            HistoryRelation::ConflictingHistories
        );
        assert_eq!(
            classify_history_relation(&singleton, &[singleton.clone(), singleton.clone()]).unwrap(),
            HistoryRelation::ConflictingHistories
        );
        assert_eq!(
            classify_history_relation(&singleton, &[singleton.clone(), disjoint]).unwrap(),
            HistoryRelation::ConflictingHistories
        );
    }

    #[test]
    fn terminal_event_classifier_distinguishes_missing_conflict_and_complete() {
        let terminal = checkpoint(1, "routine-cont-sha256:event", &[]);
        let event = super::super::super::terminal_semantic_event_from_checkpoint(
            Path::new(terminal.target()),
            &terminal,
        )
        .unwrap();
        let expected = vec![(event.clone(), true)];
        assert_eq!(
            classify_terminal_event_rows(&expected, std::slice::from_ref(&event)),
            TerminalEventAssessment::Complete
        );
        assert_eq!(
            classify_terminal_event_rows(&expected, &[]),
            TerminalEventAssessment::MissingRequired
        );
        assert_eq!(
            classify_terminal_event_rows(&[(event.clone(), false)], &[]),
            TerminalEventAssessment::Complete
        );
        assert_eq!(
            classify_terminal_event_rows(&expected, &[event.clone(), event.clone()]),
            TerminalEventAssessment::Conflicting
        );
        assert_eq!(
            classify_terminal_event_rows(
                &[(event.clone(), true), (event.clone(), true)],
                &[event.clone()]
            ),
            TerminalEventAssessment::Complete
        );
        let mut unexpected = event.clone();
        unexpected.event_id.push_str("-unexpected");
        assert_eq!(
            classify_terminal_event_rows(&expected, &[unexpected]),
            TerminalEventAssessment::Conflicting
        );

        let expected_leaves = BTreeSet::from(["expected".to_owned()]);
        assert_eq!(
            classify_terminal_leaf_sets(&expected_leaves, &BTreeSet::new()),
            TerminalEventAssessment::MissingRequired
        );
        assert_eq!(
            classify_terminal_leaf_sets(
                &expected_leaves,
                &BTreeSet::from(["unexpected".to_owned()]),
            ),
            TerminalEventAssessment::Conflicting
        );
        assert_eq!(
            classify_terminal_leaf_sets(&expected_leaves, &expected_leaves),
            TerminalEventAssessment::Complete
        );
    }

    #[test]
    fn terminal_event_holds_are_causal_private_and_planless() {
        for (assessment, status, relation) in [
            (
                TerminalEventAssessment::MissingRequired,
                "terminal_event_history_incomplete_preserve_and_hold",
                "terminal_event_history_incomplete",
            ),
            (
                TerminalEventAssessment::Conflicting,
                "terminal_event_history_conflicting_preserve_and_hold",
                "terminal_event_history_conflicting",
            ),
        ] {
            let held = terminal_event_hold(1, 37, 8, assessment);
            assert_eq!(held.status, status);
            assert_eq!(held.history_relation, relation);
            assert_eq!(held.legacy_singleton_count, 1);
            assert_eq!(held.canonical_continuation_count, 37);
            assert_eq!(held.event_journal_count, 8);
            assert!(held.reserved_recovery.is_none());
            assert!(held.quarantine_plan.is_none());
            assert!(held.next_action.contains("preserve all bytes"));
            assert!(held.next_action.contains("without synthesizing"));
            assert!(!held.next_action.contains("/private/"));
            assert!(!held.next_action.contains("sha256:"));
            assert!(!held.next_action.contains("migrate apply --plan"));
        }
    }

    #[test]
    fn explicit_abandonment_mints_only_consequence_bound_v6_plans_for_irreconcilable_history() {
        let inventory = raw_duplicate_inventory();
        let target = Path::new("/private/tmp/target");
        for assessment in [
            TerminalEventAssessment::MissingRequired,
            TerminalEventAssessment::Conflicting,
        ] {
            let ordinary = terminal_event_hold(1, 1, 0, assessment);
            assert!(ordinary.quarantine_plan.is_none());

            let admitted =
                terminal_event_abandonment_admission(1, 1, 0, assessment, &inventory, target)
                    .unwrap();
            let plan = admitted.quarantine_plan.as_ref().unwrap();
            assert_eq!(plan.schema_version, ABANDONMENT_PLAN_SCHEMA);
            assert_eq!(
                plan.authoritative_history,
                "none_explicit_operator_abandonment"
            );
            assert!(quarantine_plan_is_semantically_valid(plan));
        }

        let history = || ReconciledHistory {
            relation: HistoryRelation::ConflictingHistories,
            reserved: None,
            terminal_events: TerminalEventAssessment::Complete,
        };
        let ordinary = reconciled_hold(
            1,
            2,
            2,
            history(),
            &inventory,
            target,
            LegacyAdmissionMode::Migration,
        )
        .unwrap();
        assert!(ordinary.quarantine_plan.is_none());

        let explicit = reconciled_hold(
            1,
            2,
            2,
            history(),
            &inventory,
            target,
            LegacyAdmissionMode::ExplicitAbandonment,
        )
        .unwrap();
        let plan = explicit.quarantine_plan.as_ref().unwrap();
        assert_eq!(plan.schema_version, ABANDONMENT_PLAN_SCHEMA);
        assert_eq!(plan.history_relation, "conflicting_histories_hold");
        assert!(quarantine_plan_is_semantically_valid(plan));
    }

    #[test]
    fn stale_legacy_authentication_is_closed_to_terminal_or_reconciled_history() {
        let terminal = checkpoint(1, "routine-cont-sha256:terminal", &[]);
        assert!(may_authenticate_stale_legacy_checkpoint(&terminal));

        let mut reconciled = serde_json::to_value(&terminal).unwrap();
        reconciled["state"] = serde_json::json!("reconciled");
        reconciled["terminal_outcome"] = serde_json::Value::Null;
        let reconciled: ContinuationCheckpoint = serde_json::from_value(reconciled).unwrap();
        assert!(may_authenticate_stale_legacy_checkpoint(&reconciled));

        for (state, outcome) in [
            ("reserved", None),
            ("ambiguous", Some("ambiguous")),
            ("reconciled", Some("complete")),
            ("terminal-event-joined", None),
        ] {
            let mut rejected = serde_json::to_value(&terminal).unwrap();
            rejected["state"] = serde_json::json!(state);
            rejected["terminal_outcome"] = outcome
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
            let rejected: ContinuationCheckpoint = serde_json::from_value(rejected).unwrap();
            assert!(
                !may_authenticate_stale_legacy_checkpoint(&rejected),
                "unexpected stale authentication allowance for {state}/{outcome:?}"
            );
        }
    }

    #[test]
    fn quarantine_plan_is_exact_deterministic_and_never_minted_for_conflict() {
        let mut observed = BTreeMap::new();
        observed.insert(
            "state".to_owned(),
            InventoryEntry {
                identity: Identity {
                    device: 1,
                    inode: 2,
                    owner: 501,
                    mode: 0o40700,
                    links: 5,
                },
                content_sha256: None,
            },
        );
        observed.insert(
            "adapter/routine-continuation.json".to_owned(),
            InventoryEntry {
                identity: Identity {
                    device: 1,
                    inode: 3,
                    owner: 501,
                    mode: 0o100600,
                    links: 1,
                },
                content_sha256: Some(Sha256::digest(b"checkpoint").into()),
            },
        );

        let target = Path::new("/private/tmp/target");
        let first = quarantine_plan(&observed, "redundant_equivalent", target).unwrap();
        let second = quarantine_plan(&observed, "redundant_equivalent", target).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.schema_version, "RoutineStateQuarantinePlan-v5");
        assert!(quarantine_plan_is_semantically_valid(&first));
        let mut legacy_v4 = first.clone();
        legacy_v4.schema_version = "RoutineStateQuarantinePlan-v4".to_owned();
        assert!(!quarantine_plan_is_semantically_valid(&legacy_v4));
        assert_ne!(
            first.source_inventory_sha256,
            inventory_sha256_entries(
                observed.iter(),
                b"routine-state-quarantine-source-inventory-v2\0",
            )
        );
        for mutation in ["plan_id", "quarantine_owner", "strategy", "operation"] {
            let mut invalid = first.clone();
            match mutation {
                "plan_id" => invalid.plan_id.push('0'),
                "quarantine_owner" => invalid.quarantine_owner.push('0'),
                "strategy" => invalid.strategy.push('0'),
                "operation" => invalid.operations[0].push('0'),
                _ => unreachable!(),
            }
            assert!(
                !quarantine_plan_is_semantically_valid(&invalid),
                "{mutation}"
            );
        }
        let encoded = serde_json::to_value(&first).unwrap();
        assert_eq!(
            serde_json::from_value::<RoutineStateQuarantinePlan>(encoded.clone()).unwrap(),
            first
        );
        let mut extended = encoded;
        extended["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<RoutineStateQuarantinePlan>(extended).is_err());
        assert_eq!(
            first
                .operations
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec![
                "acquire_parent_migration_lock_and_legacy_adapter_lock_then_revalidate_exact_source_inventory",
                "stage_and_verify_fresh_v8_owner_with_plan_bound_transition_marker_without_importing_legacy_authority",
                "atomically_exchange_legacy_source_owner_with_verified_fresh_v8_stage",
                "fsync_owner_parent_after_atomic_exchange",
                "finalize_swapped_legacy_stage_to_bound_quarantine_sibling",
                "fsync_owner_parent_after_quarantine_finalize",
                "verify_active_fresh_v8_transition_and_unchanged_quarantine_inventory",
                "stage_and_fsync_exact_plan_bound_settlement_receipt",
                "atomically_publish_settlement_receipt_and_fsync_active_fresh_v8_owner",
                "verify_permanent_pending_and_settled_receipts_with_active_fresh_v8_and_unchanged_quarantine",
            ]
        );
        assert_eq!(
            first.strategy,
            "stage_fresh_v8_then_atomic_exchange_finalize_legacy_and_publish_settlement_receipt"
        );
        assert!(first.rollback.contains("before_swap"));
        assert!(first.rollback.contains("after_swap"));
        assert!(first.rollback.contains("permanent pending record"));
        assert_eq!(first.source_owner, "routine-public");
        assert_eq!(first.apply_capability, "available_exact_record_only");
        assert!(first.source_owner_tree_sha256.starts_with("sha256:"));
        assert!(first.target_path_sha256.starts_with("sha256:"));
        assert!(
            first
                .quarantine_owner
                .starts_with("routine-public.quarantine-")
        );
        assert!(quarantine_plan(&observed, "conflicting_histories_hold", target).is_err());

        observed.get_mut("state").unwrap().identity.inode += 1;
        let changed = quarantine_plan(&observed, "redundant_equivalent", target).unwrap();
        assert_ne!(first.plan_id, changed.plan_id);
        assert_ne!(
            first.source_inventory_sha256,
            changed.source_inventory_sha256
        );

        observed.get_mut("state").unwrap().identity.inode -= 1;
        observed.get_mut("state").unwrap().identity.device += 9;
        let renumbered_device = quarantine_plan(&observed, "redundant_equivalent", target).unwrap();
        assert_eq!(first.plan_id, renumbered_device.plan_id);
        assert_eq!(
            first.source_inventory_sha256,
            renumbered_device.source_inventory_sha256
        );
        assert_eq!(
            first.source_owner_tree_sha256,
            renumbered_device.source_owner_tree_sha256
        );

        observed
            .get_mut("adapter/routine-continuation.json")
            .unwrap()
            .content_sha256 = Some(Sha256::digest(b"rewritten-in-place").into());
        let rewritten = quarantine_plan(&observed, "redundant_equivalent", target).unwrap();
        assert_ne!(first.plan_id, rewritten.plan_id);

        let other_target = quarantine_plan(
            &observed,
            "redundant_equivalent",
            Path::new("/private/tmp/other"),
        )
        .unwrap();
        assert_ne!(first.target_path_sha256, other_target.target_path_sha256);
        assert_ne!(first.plan_id, other_target.plan_id);

        let held = reconciled_hold(
            1,
            1,
            0,
            ReconciledHistory {
                relation: HistoryRelation::ConflictingHistories,
                reserved: Some(RoutineReservedRecoveryAssessment {
                    owner: crate::routine_work::ReservedRecoveryOwnerObservation::NotObserved,
                    effect: crate::routine_work::ReservedRecoveryEffectEvidence::PristineNoEffect,
                }),
                terminal_events: TerminalEventAssessment::Complete,
            },
            &observed,
            target,
            LegacyAdmissionMode::Migration,
        )
        .unwrap();
        assert!(held.quarantine_plan.is_none());
        assert_eq!(held.history_relation, "conflicting_histories_hold");
        assert!(held.reserved_recovery.is_none());
    }

    #[test]
    fn unrelated_owner_parent_siblings_never_enter_the_inventory() {
        let root = std::env::temp_dir().join(format!(
            "hmi{:x}{:x}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        create_empty_legacy_owner(&root);
        let baseline = test_inventory(&root, STATE_COMPONENTS[3], &[]).unwrap();

        let unreadable = root.join("unrelated-regular");
        fs::write(&unreadable, b"must not be opened").unwrap();
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();
        let unrelated_directory = root.join("repository-fit");
        fs::create_dir(&unrelated_directory).unwrap();
        fs::set_permissions(&unrelated_directory, fs::Permissions::from_mode(0o777)).unwrap();
        symlink("missing-target", root.join("unrelated-symlink")).unwrap();
        let fifo = root.join("unrelated-fifo");
        let fifo_name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        // SAFETY: `fifo_name` is a live NUL-terminated pathname and the mode is bounded.
        assert_eq!(unsafe { libc::mkfifo(fifo_name.as_ptr(), 0o000) }, 0);

        let with_siblings = test_inventory(&root, STATE_COMPONENTS[3], &[]).unwrap();
        assert_eq!(with_siblings, baseline);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reserved_parent_namespace_is_exact_and_ignored_identity_remains_snapshotted() {
        let root = std::env::temp_dir().join(format!(
            "hul-migration-reserved-parent-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        create_empty_legacy_owner(&root);
        let baseline = test_inventory(&root, STATE_COMPONENTS[3], &[]).unwrap();
        let bootstrap = root.join(BOOTSTRAP_STAGE);

        fs::write(&bootstrap, b"wrong type").unwrap();
        assert!(test_inventory(&root, STATE_COMPONENTS[3], &[BOOTSTRAP_STAGE]).is_err());
        fs::remove_file(&bootstrap).unwrap();
        symlink(STATE_COMPONENTS[3], &bootstrap).unwrap();
        assert!(test_inventory(&root, STATE_COMPONENTS[3], &[BOOTSTRAP_STAGE]).is_err());
        fs::remove_file(&bootstrap).unwrap();
        fs::create_dir(&bootstrap).unwrap();
        fs::set_permissions(&bootstrap, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(test_inventory(&root, STATE_COMPONENTS[3], &[BOOTSTRAP_STAGE]).is_err());
        fs::set_permissions(&bootstrap, fs::Permissions::from_mode(0o700)).unwrap();

        let before = test_inventory(&root, STATE_COMPONENTS[3], &[BOOTSTRAP_STAGE]).unwrap();
        assert_ne!(before, baseline);
        assert_eq!(inventory_sha256(&before), inventory_sha256(&baseline));
        assert_eq!(owner_tree_sha256(&before), owner_tree_sha256(&baseline));

        let extra = root.join("routine-public.quarantine-extra");
        fs::create_dir(&extra).unwrap();
        fs::set_permissions(&extra, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(test_inventory(&root, STATE_COMPONENTS[3], &[BOOTSTRAP_STAGE]).is_err());
        fs::remove_dir(&extra).unwrap();

        let drift = bootstrap.join("identity-drift");
        fs::create_dir(&drift).unwrap();
        fs::set_permissions(&drift, fs::Permissions::from_mode(0o700)).unwrap();
        let after = test_inventory(&root, STATE_COMPONENTS[3], &[BOOTSTRAP_STAGE]).unwrap();
        assert_ne!(after, before);
        assert_eq!(inventory_sha256(&after), inventory_sha256(&before));
        assert_eq!(owner_tree_sha256(&after), owner_tree_sha256(&before));

        fs::remove_dir(&drift).unwrap();
        fs::remove_dir(&bootstrap).unwrap();
        assert!(test_inventory(&root, STATE_COMPONENTS[3], &[BOOTSTRAP_STAGE]).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn active_bootstrap_and_quarantine_replay_normalize_the_same_legacy_source() {
        let root = std::env::temp_dir().join(format!(
            "hul-migration-transition-normalization-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        create_empty_legacy_owner(&root);
        let baseline = test_inventory(&root, STATE_COMPONENTS[3], &[]).unwrap();

        let bootstrap = root.join(BOOTSTRAP_STAGE);
        fs::create_dir(&bootstrap).unwrap();
        fs::set_permissions(&bootstrap, fs::Permissions::from_mode(0o700)).unwrap();
        let staged = test_inventory(&root, STATE_COMPONENTS[3], &[BOOTSTRAP_STAGE]).unwrap();
        assert_eq!(inventory_sha256(&staged), inventory_sha256(&baseline));

        let swap = root.join("unrelated-swap-name");
        fs::rename(root.join(STATE_COMPONENTS[3]), &swap).unwrap();
        fs::rename(&bootstrap, root.join(STATE_COMPONENTS[3])).unwrap();
        fs::rename(&swap, &bootstrap).unwrap();
        let exchanged = test_inventory(&root, BOOTSTRAP_STAGE, &[STATE_COMPONENTS[3]]).unwrap();
        assert_eq!(inventory_sha256(&exchanged), inventory_sha256(&baseline));
        assert_eq!(owner_tree_sha256(&exchanged), owner_tree_sha256(&baseline));

        let quarantine_name = "routine-public.quarantine-test-plan";
        fs::rename(&bootstrap, root.join(quarantine_name)).unwrap();
        let quarantined = test_inventory(&root, quarantine_name, &[STATE_COMPONENTS[3]]).unwrap();
        assert_eq!(inventory_sha256(&quarantined), inventory_sha256(&baseline));
        assert_eq!(
            owner_tree_sha256(&quarantined),
            owner_tree_sha256(&baseline)
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn same_inode_content_rewrite_invalidates_the_observed_inventory() {
        let root = std::env::temp_dir().join(format!(
            "hul-migration-inventory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let state_path = root.join(STATE_COMPONENTS[3]);
        let authority_path = state_path.join(AUTHORITY_DIRECTORY);
        let adapter_path = state_path.join(ADAPTER_DIRECTORY);
        let launch_path = state_path.join(LAUNCH_DIRECTORY);
        for directory in [
            &root,
            &state_path,
            &authority_path,
            &adapter_path,
            &launch_path,
        ] {
            fs::create_dir(directory).unwrap();
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let event = adapter_path.join(format!(
            "{EVENT_FILE_PREFIX}{}{EVENT_FILE_SUFFIX}",
            "a".repeat(64)
        ));
        fs::write(&event, b"first\n").unwrap();
        fs::set_permissions(&event, fs::Permissions::from_mode(0o600)).unwrap();

        let owner_parent =
            AnchoredDirectory::open_absolute(&root, DirectorySecurity::PrivateAuthority).unwrap();
        let state =
            AnchoredDirectory::open_absolute(&state_path, DirectorySecurity::PrivateAuthority)
                .unwrap();
        let authority = state.open_child(AUTHORITY_DIRECTORY).unwrap();
        let adapter = state.open_child(ADAPTER_DIRECTORY).unwrap();
        let launch = state.open_child(LAUNCH_DIRECTORY).unwrap();
        let before = inventory(
            &owner_parent,
            &state,
            &authority,
            &adapter,
            &launch,
            STATE_COMPONENTS[3],
            &[],
        )
        .unwrap();

        let original_inode = fs::metadata(&event).unwrap().ino();
        fs::write(&event, b"replacement\n").unwrap();
        fs::set_permissions(&event, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(fs::metadata(&event).unwrap().ino(), original_inode);
        let after = inventory(
            &owner_parent,
            &state,
            &authority,
            &adapter,
            &launch,
            STATE_COMPONENTS[3],
            &[],
        )
        .unwrap();

        assert_ne!(before, after);
        drop((launch, adapter, authority, state, owner_parent));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ignored_transition_directory_normalizes_the_parent_link_count() {
        let root = std::env::temp_dir().join(format!(
            "hul-migration-parent-links-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let state_path = root.join(STATE_COMPONENTS[3]);
        let authority_path = state_path.join(AUTHORITY_DIRECTORY);
        let adapter_path = state_path.join(ADAPTER_DIRECTORY);
        let launch_path = state_path.join(LAUNCH_DIRECTORY);
        for directory in [
            &root,
            &state_path,
            &authority_path,
            &adapter_path,
            &launch_path,
        ] {
            fs::create_dir(directory).unwrap();
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let owner_parent =
            AnchoredDirectory::open_absolute(&root, DirectorySecurity::PrivateAuthority).unwrap();
        let state =
            AnchoredDirectory::open_absolute(&state_path, DirectorySecurity::PrivateAuthority)
                .unwrap();
        let authority = state.open_child(AUTHORITY_DIRECTORY).unwrap();
        let adapter = state.open_child(ADAPTER_DIRECTORY).unwrap();
        let launch = state.open_child(LAUNCH_DIRECTORY).unwrap();
        let baseline = inventory(
            &owner_parent,
            &state,
            &authority,
            &adapter,
            &launch,
            STATE_COMPONENTS[3],
            &[],
        )
        .unwrap();

        let fresh = root.join(BOOTSTRAP_STAGE);
        fs::create_dir(&fresh).unwrap();
        fs::set_permissions(&fresh, fs::Permissions::from_mode(0o700)).unwrap();
        let normalized = inventory(
            &owner_parent,
            &state,
            &authority,
            &adapter,
            &launch,
            STATE_COMPONENTS[3],
            &[BOOTSTRAP_STAGE],
        )
        .unwrap();
        assert_ne!(normalized, baseline);
        assert!(normalized.contains_key(&format!(
            "{IGNORED_PARENT_SNAPSHOT_PREFIX}{BOOTSTRAP_STAGE}"
        )));
        assert_eq!(inventory_sha256(&normalized), inventory_sha256(&baseline));
        assert_eq!(owner_tree_sha256(&normalized), owner_tree_sha256(&baseline));

        drop((launch, adapter, authority, state, owner_parent));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_quarantine_sibling_is_still_a_preserve_and_hold_transition() {
        let root = std::env::temp_dir().join(format!(
            "hul-malformed-quarantine-transition-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let malformed = root.join("routine-public.quarantine-not-a-plan-id");
        fs::create_dir(&malformed).unwrap();
        fs::set_permissions(&malformed, fs::Permissions::from_mode(0o700)).unwrap();
        let parent =
            AnchoredDirectory::open_absolute(&root, DirectorySecurity::PrivateAuthority).unwrap();

        assert_eq!(
            transition_entries(&parent).unwrap(),
            BTreeSet::from(["routine-public.quarantine-not-a-plan-id".to_owned()])
        );

        drop(parent);
        fs::remove_dir_all(root).unwrap();
    }
}
