use super::super::{CheckpointBinding, RoutineStateMigrationAdmission, RoutineStateQuarantinePlan};
use super::*;
use crate::routine_work::{RoutineCustodyCapability, RoutineReservedRecoveryAssessment};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const AUTHORITY_ENTRIES: &[&str] = &[
    "routine-authority.key",
    "routine-authority.lock",
    "routine-authority.state",
];
const MAX_AUTHORITY_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_EVENT_ENTRY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CHECKPOINT_ENTRY_BYTES: u64 = 16 * 1024;
const MAX_LOCK_ENTRY_BYTES: u64 = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InventoryEntry {
    identity: Identity,
    content_sha256: Option<[u8; 32]>,
}

pub(crate) fn assess_migration_admission(
    home: &Path,
    binding: CheckpointBinding<'_>,
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
    let state = match harness.open_child(STATE_COMPONENTS[3]) {
        Ok(value) => value,
        Err(HostFailure::Unavailable) => return Ok(None),
        Err(error) => return Err(error),
    };

    if state.stat(STATE_FORMAT_NAME)?.is_some() {
        return assess_versioned(home, state).map(Some);
    }
    assess_format_absent(&harness, state, binding).map(Some)
}

fn assess_versioned(
    home: AnchoredDirectory,
    state: AnchoredDirectory,
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
    match host_state::open_existing_state(home, state) {
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

fn assess_format_absent(
    owner_parent: &AnchoredDirectory,
    state: AnchoredDirectory,
    binding: CheckpointBinding<'_>,
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

    let before = inventory(owner_parent, &state, &authority, &adapter, &launch)?;
    let (singleton_count, continuation_count, event_count) = counts(&adapter)?;
    if singleton_count == 1 && continuation_count > 0 {
        let relation = match reconcile_mixed_history(&authority, &adapter, &launch, binding) {
            Ok(value) => value,
            Err(_) => return Ok(unsafe_hold("absent")),
        };
        let after = inventory(owner_parent, &state, &authority, &adapter, &launch)?;
        if before != after {
            return Ok(stale_writer_hold("absent"));
        }
        return reconciled_hold(
            singleton_count,
            continuation_count,
            event_count,
            relation,
            &before,
        );
    }
    if validate_legacy(&adapter).is_err() {
        return Ok(unsafe_hold("absent"));
    }
    let after = inventory(owner_parent, &state, &authority, &adapter, &launch)?;
    if before != after {
        return Ok(stale_writer_hold("absent"));
    }
    let quarantine_plan = quarantine_plan(&before, "single_history_only")?;
    Ok(RoutineStateMigrationAdmission {
        status: "migration_candidate_requires_approval",
        format_status: "absent_legacy",
        legacy_singleton_count: singleton_count,
        canonical_continuation_count: continuation_count,
        event_journal_count: event_count,
        history_relation: "single_history_only",
        next_action: "independently review the deterministic reversible quarantine plan before any host write",
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
    validate_terminal_event_history(adapter, &checkpoints)?;
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
) -> Result<(), HostFailure> {
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
    if actual_leaves != expected.keys().cloned().collect() {
        return Err(HostFailure::Invalid);
    }
    for (leaf, expected_events) in expected {
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
        let query = crate::observability::EventQuery::new(
            binding.context_id(),
            binding.candidate_id(),
            binding.source_id(),
        )
        .map_err(|_| HostFailure::Invalid)?
        .limit(crate::observability::EventStore::supported_result_limit())
        .map_err(|_| HostFailure::Invalid)?;
        let events = store.query(&query).map_err(|_| HostFailure::Invalid)?;
        if events.iter().any(|event| {
            !expected_events
                .iter()
                .any(|(expected, _)| expected == event)
        }) || expected_events.iter().any(|(expected, required)| {
            let count = events.iter().filter(|event| *event == expected).count();
            count > 1 || (*required && count != 1)
        }) {
            return Err(HostFailure::Invalid);
        }
    }
    Ok(())
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
) -> Result<BTreeMap<String, InventoryEntry>, HostFailure> {
    let mut rows = BTreeMap::new();
    rows.insert("owner_parent".to_owned(), directory_inventory(owner_parent));
    rows.insert("state".to_owned(), directory_inventory(state));
    rows.insert("authority".to_owned(), directory_inventory(authority));
    rows.insert("adapter".to_owned(), directory_inventory(adapter));
    rows.insert("launch".to_owned(), directory_inventory(launch));
    capture_entries("owner_parent", owner_parent, &mut rows)?;
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

fn quarantine_plan(
    inventory: &BTreeMap<String, InventoryEntry>,
    history_relation: &'static str,
) -> Result<RoutineStateQuarantinePlan, HostFailure> {
    let authoritative_history = match history_relation {
        "single_history_only" => "the_only_authenticated_history",
        "redundant_equivalent" | "canonical_strictly_supersedes" => {
            "canonical_authenticated_history"
        }
        "singleton_strictly_supersedes" => "singleton_authenticated_history",
        _ => return Err(HostFailure::Invalid),
    };
    let source_inventory_sha256 = inventory_sha256(inventory);
    let mut plan = Sha256::new();
    plan.update(b"routine-state-quarantine-plan-v1\0");
    plan.update(source_inventory_sha256.as_bytes());
    plan.update([0]);
    plan.update(history_relation.as_bytes());
    plan.update(b"\0quarantine-entire-legacy-owner-then-bootstrap-v8");
    let plan_id = format!("routine-quarantine-sha256:{:x}", plan.finalize());
    let suffix = plan_id
        .strip_prefix("routine-quarantine-sha256:")
        .ok_or(HostFailure::Invalid)?
        .to_owned();
    Ok(RoutineStateQuarantinePlan {
        plan_id,
        source_inventory_sha256,
        history_relation,
        authoritative_history,
        source_owner: STATE_COMPONENTS[3],
        quarantine_owner: format!("{}.quarantine-{}", STATE_COMPONENTS[3], suffix),
        target_format: "routine-host-state-v8",
        strategy: "quarantine_entire_legacy_owner_then_bootstrap_v8",
        apply_capability: "not_implemented",
        operations: vec![
            "acquire_parent_migration_lock_and_revalidate_exact_source_inventory",
            "atomically_rename_source_owner_to_quarantine_sibling",
            "fsync_owner_parent_after_quarantine_publish",
            "bootstrap_fresh_v8_owner_without_importing_legacy_authority",
            "verify_fresh_v8_owner_and_unchanged_quarantine_inventory",
        ],
        rollback: "only_before_any_new_v8_attempt: quarantine the verified-empty v8 owner, atomically restore the unchanged source owner, and fsync the owner parent",
    })
}

fn inventory_sha256(inventory: &BTreeMap<String, InventoryEntry>) -> String {
    let mut digest = Sha256::new();
    digest.update(b"routine-state-quarantine-source-inventory-v1\0");
    for (path, identity) in inventory {
        digest.update(path.as_bytes());
        digest.update([0]);
        digest.update(identity.identity.device.to_le_bytes());
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
) -> Result<RoutineStateMigrationAdmission, HostFailure> {
    let relation = history.relation;
    if relation == HistoryRelation::RedundantEquivalent
        && raw_duplicate_history(inventory)
        && let Some(assessment) = history.reserved
    {
        let (reserved_recovery, next_action) = reserved_recovery_admission::admission(assessment);
        return Ok(RoutineStateMigrationAdmission {
            status: reserved_recovery.status,
            format_status: "absent_legacy",
            legacy_singleton_count: singleton,
            canonical_continuation_count: canonical,
            event_journal_count: events,
            history_relation: relation.as_str(),
            next_action,
            reserved_recovery: Some(reserved_recovery),
            quarantine_plan: None,
        });
    }
    let quarantine_plan = match relation {
        HistoryRelation::RedundantEquivalent
        | HistoryRelation::CanonicalStrictlySupersedes
        | HistoryRelation::SingletonStrictlySupersedes => {
            Some(quarantine_plan(inventory, relation.as_str())?)
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
        next_action: if quarantine_plan.is_some() {
            "preserve all bytes; independently review the deterministic reversible quarantine plan before any host write"
        } else {
            "preserve all bytes; resolve the conflicting authenticated histories without proposing a migration"
        },
        reserved_recovery: None,
        quarantine_plan,
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
    canonical
        .iter()
        .filter(|candidate| **candidate == singleton)
        .count()
        == 1
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
    use std::os::unix::fs::PermissionsExt;
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
        let identity = Identity {
            device: 1,
            inode: 2,
            owner: 501,
            mode: 0o100600,
            links: 1,
        };
        let digest: [u8; 32] = Sha256::digest(b"same raw checkpoint").into();
        let mut inventory = BTreeMap::from([
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
        ]);
        assert!(raw_duplicate_history(&inventory));

        inventory
            .get_mut("adapter/continuations/routine-continuation-a.json")
            .unwrap()
            .content_sha256 = Some(Sha256::digest(b"semantic match, raw difference").into());
        assert!(!raw_duplicate_history(&inventory));

        inventory
            .get_mut("adapter/continuations/routine-continuation-a.json")
            .unwrap()
            .content_sha256 = Some(digest);
        inventory.insert(
            "adapter/continuations/routine-continuation-b.json".to_owned(),
            InventoryEntry {
                identity,
                content_sha256: Some(digest),
            },
        );
        assert!(!raw_duplicate_history(&inventory));
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

        let first = quarantine_plan(&observed, "redundant_equivalent").unwrap();
        let second = quarantine_plan(&observed, "redundant_equivalent").unwrap();
        assert_eq!(first, second);
        assert_eq!(first.operations.len(), 5);
        assert_eq!(first.source_owner, "routine-public");
        assert!(
            first
                .quarantine_owner
                .starts_with("routine-public.quarantine-")
        );
        assert!(quarantine_plan(&observed, "conflicting_histories_hold").is_err());

        observed.get_mut("state").unwrap().identity.inode += 1;
        let changed = quarantine_plan(&observed, "redundant_equivalent").unwrap();
        assert_ne!(first.plan_id, changed.plan_id);
        assert_ne!(
            first.source_inventory_sha256,
            changed.source_inventory_sha256
        );

        observed.get_mut("state").unwrap().identity.inode -= 1;
        observed
            .get_mut("adapter/routine-continuation.json")
            .unwrap()
            .content_sha256 = Some(Sha256::digest(b"rewritten-in-place").into());
        let rewritten = quarantine_plan(&observed, "redundant_equivalent").unwrap();
        assert_ne!(first.plan_id, rewritten.plan_id);

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
            },
            &observed,
        )
        .unwrap();
        assert!(held.quarantine_plan.is_none());
        assert_eq!(held.history_relation, "conflicting_histories_hold");
        assert!(held.reserved_recovery.is_none());
    }

    #[test]
    fn same_inode_content_rewrite_invalidates_the_observed_inventory() {
        let root = std::env::temp_dir().join(format!(
            "hul-migration-inventory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let state_path = root.join("state");
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
        let before = inventory(&owner_parent, &state, &authority, &adapter, &launch).unwrap();

        let original_inode = fs::metadata(&event).unwrap().ino();
        fs::write(&event, b"replacement\n").unwrap();
        fs::set_permissions(&event, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(fs::metadata(&event).unwrap().ino(), original_inode);
        let after = inventory(&owner_parent, &state, &authority, &adapter, &launch).unwrap();

        assert_ne!(before, after);
        drop((launch, adapter, authority, state, owner_parent));
        fs::remove_dir_all(root).unwrap();
    }
}
