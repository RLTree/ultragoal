use super::super::RoutineStateQuarantinePlan;
use super::*;
use std::ffi::CString;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;

const RECEIPT_LIMIT: u64 = 64 * 1024;
const NOFOLLOW_AND_BENEATH: libc::c_uint = 0x10 | 0x20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SettlementState {
    None,
    Pending,
    ReceiptStaged,
    Settled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SettlementPublication {
    DurableSettled,
    ReceiptVisibleDurabilityUnacknowledged,
    StillPending,
    Invalid,
}

#[cfg(test)]
thread_local! {
    static BEFORE_PUBLICATION_FAILPOINT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static AFTER_RENAME_FAILPOINT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ACKNOWLEDGEMENT_FAILPOINT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static SETTLED_REPLAY_EVIDENCE_FAILPOINT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(super) fn fail_before_next_receipt_publication() {
    BEFORE_PUBLICATION_FAILPOINT.with(|failpoint| failpoint.set(true));
}

#[cfg(test)]
pub(super) fn fail_after_next_receipt_rename() {
    AFTER_RENAME_FAILPOINT.with(|failpoint| failpoint.set(true));
}

#[cfg(test)]
pub(super) fn fail_next_receipt_acknowledgement() {
    ACKNOWLEDGEMENT_FAILPOINT.with(|failpoint| failpoint.set(true));
}

#[cfg(test)]
pub(super) fn fail_next_settled_replay_evidence_check() {
    SETTLED_REPLAY_EVIDENCE_FAILPOINT.with(|failpoint| failpoint.set(true));
}

#[cfg(test)]
fn take_before_publication_failpoint() -> bool {
    BEFORE_PUBLICATION_FAILPOINT.with(|failpoint| failpoint.replace(false))
}

#[cfg(not(test))]
fn take_before_publication_failpoint() -> bool {
    false
}

#[cfg(test)]
fn take_after_rename_failpoint() -> bool {
    AFTER_RENAME_FAILPOINT.with(|failpoint| failpoint.replace(false))
}

#[cfg(test)]
fn take_acknowledgement_failpoint() -> bool {
    ACKNOWLEDGEMENT_FAILPOINT.with(|failpoint| failpoint.replace(false))
}

#[cfg(test)]
fn take_settled_replay_evidence_failpoint() -> bool {
    SETTLED_REPLAY_EVIDENCE_FAILPOINT.with(|failpoint| failpoint.replace(false))
}

#[cfg(not(test))]
fn take_acknowledgement_failpoint() -> bool {
    false
}

#[cfg(not(test))]
fn take_after_rename_failpoint() -> bool {
    false
}

pub(super) fn begin(
    state: &AnchoredDirectory,
    accepted: &RoutineStateQuarantinePlan,
) -> Result<(), HostFailure> {
    let expected = canonical_plan(accepted)?;
    match classify_expected(state, &expected)? {
        SettlementState::None => {
            create_exact(state, QUARANTINE_TRANSITION_MARKER, &expected)?;
            state.file.sync_all().map_err(|_| HostFailure::Invalid)?;
        }
        SettlementState::Pending | SettlementState::ReceiptStaged | SettlementState::Settled => {}
    }
    verify(state, accepted)
}

pub(super) fn is_present(state: &AnchoredDirectory) -> Result<bool, HostFailure> {
    Ok(state.stat(QUARANTINE_TRANSITION_MARKER)?.is_some()
        || state.stat(QUARANTINE_SETTLEMENT_STAGE)?.is_some()
        || state.stat(QUARANTINE_SETTLEMENT_RECEIPT)?.is_some())
}

pub(super) fn classify(
    state: &AnchoredDirectory,
    accepted: &RoutineStateQuarantinePlan,
) -> Result<SettlementState, HostFailure> {
    classify_expected(state, &canonical_plan(accepted)?)
}

pub(super) fn classify_present(
    state: &AnchoredDirectory,
) -> Result<Option<(RoutineStateQuarantinePlan, SettlementState)>, HostFailure> {
    if !is_present(state)? {
        return Ok(None);
    }
    let bytes = read_exact_bytes(state, QUARANTINE_TRANSITION_MARKER)?;
    let accepted: RoutineStateQuarantinePlan =
        serde_json::from_slice(&bytes).map_err(|_| HostFailure::Invalid)?;
    if canonical_plan(&accepted)? != bytes {
        return Err(HostFailure::Invalid);
    }
    let settlement = classify_expected(state, &bytes)?;
    Ok(Some((accepted, settlement)))
}

pub(super) fn verify(
    state: &AnchoredDirectory,
    accepted: &RoutineStateQuarantinePlan,
) -> Result<(), HostFailure> {
    match classify(state, accepted)? {
        SettlementState::Pending | SettlementState::ReceiptStaged | SettlementState::Settled => {
            Ok(())
        }
        SettlementState::None => Err(HostFailure::Invalid),
    }
}

pub(super) fn settle(
    state: &AnchoredDirectory,
    accepted: &RoutineStateQuarantinePlan,
) -> SettlementPublication {
    let expected = match canonical_plan(accepted) {
        Ok(expected) => expected,
        Err(_) => return SettlementPublication::Invalid,
    };
    match classify_expected(state, &expected) {
        Ok(SettlementState::Settled) => return acknowledge_visible_receipt(state, &expected),
        Ok(SettlementState::Pending) => {
            if create_exact(state, QUARANTINE_SETTLEMENT_STAGE, &expected).is_err() {
                return classify_failed_publication(state, &expected);
            }
            if state.file.sync_all().is_err() {
                return SettlementPublication::StillPending;
            }
        }
        Ok(SettlementState::ReceiptStaged) => {}
        Ok(SettlementState::None) | Err(_) => return SettlementPublication::Invalid,
    }

    if take_before_publication_failpoint() {
        return SettlementPublication::StillPending;
    }
    publish_receipt(state, &expected)
}

fn classify_expected(
    state: &AnchoredDirectory,
    expected: &[u8],
) -> Result<SettlementState, HostFailure> {
    let pending = state.stat(QUARANTINE_TRANSITION_MARKER)?.is_some();
    let staged = state.stat(QUARANTINE_SETTLEMENT_STAGE)?.is_some();
    let settled = state.stat(QUARANTINE_SETTLEMENT_RECEIPT)?.is_some();

    match (pending, staged, settled) {
        (false, false, false) => Ok(SettlementState::None),
        (true, false, false) => {
            verify_exact(state, QUARANTINE_TRANSITION_MARKER, expected)?;
            Ok(SettlementState::Pending)
        }
        (true, true, false) => {
            verify_exact(state, QUARANTINE_TRANSITION_MARKER, expected)?;
            verify_exact(state, QUARANTINE_SETTLEMENT_STAGE, expected)?;
            Ok(SettlementState::ReceiptStaged)
        }
        (true, false, true) => {
            verify_exact(state, QUARANTINE_TRANSITION_MARKER, expected)?;
            verify_exact(state, QUARANTINE_SETTLEMENT_RECEIPT, expected)?;
            Ok(SettlementState::Settled)
        }
        _ => Err(HostFailure::Invalid),
    }
}

fn create_exact(state: &AnchoredDirectory, name: &str, expected: &[u8]) -> Result<(), HostFailure> {
    let (mut file, created) = state.open_or_create_regular(name, 0o600)?;
    if created {
        file.write_all(expected).map_err(|_| HostFailure::Invalid)?;
        file.sync_all().map_err(|_| HostFailure::Invalid)?;
    }
    verify_exact(state, name, expected)
}

fn verify_exact(state: &AnchoredDirectory, name: &str, expected: &[u8]) -> Result<(), HostFailure> {
    if read_exact_bytes(state, name)? != expected {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

fn read_exact_bytes(state: &AnchoredDirectory, name: &str) -> Result<Vec<u8>, HostFailure> {
    let file = state.open_regular(name, libc::O_RDONLY, 0o600)?;
    let mut observed = Vec::new();
    file.take(RECEIPT_LIMIT + 1)
        .read_to_end(&mut observed)
        .map_err(|_| HostFailure::Invalid)?;
    if observed.len() as u64 > RECEIPT_LIMIT {
        return Err(HostFailure::Invalid);
    }
    Ok(observed)
}

fn publish_receipt(state: &AnchoredDirectory, expected: &[u8]) -> SettlementPublication {
    let staged = match state.open_regular(QUARANTINE_SETTLEMENT_STAGE, libc::O_RDONLY, 0o600) {
        Ok(staged) => staged,
        Err(_) => return classify_failed_publication(state, expected),
    };
    let staged_identity = match staged.metadata() {
        Ok(metadata) => identity(&metadata),
        Err(_) => return SettlementPublication::Invalid,
    };
    if state.stat(QUARANTINE_SETTLEMENT_STAGE).ok().flatten() != Some(staged_identity)
        || state
            .stat(QUARANTINE_SETTLEMENT_RECEIPT)
            .ok()
            .flatten()
            .is_some()
        || verify_exact(state, QUARANTINE_SETTLEMENT_STAGE, expected).is_err()
    {
        return SettlementPublication::Invalid;
    }
    if staged.sync_all().is_err() {
        return SettlementPublication::StillPending;
    }

    let source = match CString::new(QUARANTINE_SETTLEMENT_STAGE) {
        Ok(source) => source,
        Err(_) => return SettlementPublication::Invalid,
    };
    let destination = match CString::new(QUARANTINE_SETTLEMENT_RECEIPT) {
        Ok(destination) => destination,
        Err(_) => return SettlementPublication::Invalid,
    };
    // SAFETY: both constant names are single validated components, both sides
    // are anchored to the same live state descriptor, the open source
    // descriptor's identity was rechecked, and RENAME_EXCL forbids replacing a
    // receipt published by another transition.
    let renamed = unsafe {
        libc::renameatx_np(
            state.file.as_raw_fd(),
            source.as_ptr(),
            state.file.as_raw_fd(),
            destination.as_ptr(),
            libc::RENAME_EXCL | NOFOLLOW_AND_BENEATH,
        )
    };
    if renamed != 0 {
        return classify_failed_publication(state, expected);
    }
    if verify_exact(state, QUARANTINE_SETTLEMENT_RECEIPT, expected).is_err() {
        return SettlementPublication::Invalid;
    }
    if take_after_rename_failpoint() || state.file.sync_all().is_err() {
        return SettlementPublication::ReceiptVisibleDurabilityUnacknowledged;
    }
    match classify_expected(state, expected) {
        Ok(SettlementState::Settled) => SettlementPublication::DurableSettled,
        _ => SettlementPublication::Invalid,
    }
}

fn acknowledge_visible_receipt(
    state: &AnchoredDirectory,
    expected: &[u8],
) -> SettlementPublication {
    if classify_expected(state, expected) != Ok(SettlementState::Settled) {
        return SettlementPublication::Invalid;
    }
    if take_acknowledgement_failpoint() || state.file.sync_all().is_err() {
        return SettlementPublication::ReceiptVisibleDurabilityUnacknowledged;
    }
    #[cfg(test)]
    if take_settled_replay_evidence_failpoint()
        && state.remove_regular(QUARANTINE_TRANSITION_MARKER).is_err()
    {
        return SettlementPublication::Invalid;
    }
    match classify_expected(state, expected) {
        Ok(SettlementState::Settled) => SettlementPublication::DurableSettled,
        _ => SettlementPublication::Invalid,
    }
}

fn classify_failed_publication(
    state: &AnchoredDirectory,
    expected: &[u8],
) -> SettlementPublication {
    match classify_expected(state, expected) {
        Ok(SettlementState::Pending | SettlementState::ReceiptStaged) => {
            SettlementPublication::StillPending
        }
        Ok(SettlementState::Settled) => {
            SettlementPublication::ReceiptVisibleDurabilityUnacknowledged
        }
        Ok(SettlementState::None) | Err(_) => SettlementPublication::Invalid,
    }
}

fn canonical_plan(accepted: &RoutineStateQuarantinePlan) -> Result<Vec<u8>, HostFailure> {
    let bytes = serde_json::to_vec(accepted).map_err(|_| HostFailure::Invalid)?;
    if bytes.len() as u64 > RECEIPT_LIMIT {
        return Err(HostFailure::Invalid);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> (
        std::path::PathBuf,
        AnchoredDirectory,
        RoutineStateQuarantinePlan,
    ) {
        let root = std::env::temp_dir().join(format!(
            "hul-quarantine-settlement-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let state =
            AnchoredDirectory::open_absolute(&root, DirectorySecurity::PrivateAuthority).unwrap();
        let plan = RoutineStateQuarantinePlan {
            schema_version: "RoutineStateQuarantinePlan-v4".to_owned(),
            plan_id: "sha256:plan".to_owned(),
            source_inventory_sha256: "sha256:inventory".to_owned(),
            source_owner_tree_sha256: "sha256:owner".to_owned(),
            target_path_sha256: "sha256:target".to_owned(),
            history_relation: "unproven".to_owned(),
            authoritative_history: "none".to_owned(),
            source_owner: "routine-public".to_owned(),
            quarantine_owner: "routine-public.quarantine-plan".to_owned(),
            target_format: "routine-host-state-v8".to_owned(),
            strategy: "quarantine".to_owned(),
            apply_capability: "migrate_apply".to_owned(),
            operations: vec!["stage".to_owned(), "publish".to_owned()],
            rollback: "preserve".to_owned(),
        };
        (root, state, plan)
    }

    fn exact_bytes(root: &std::path::Path, name: &str, plan: &RoutineStateQuarantinePlan) {
        assert_eq!(
            std::fs::read(root.join(name)).unwrap(),
            serde_json::to_vec(plan).unwrap()
        );
    }

    #[test]
    fn begin_persists_exact_plan_bound_pending_record() {
        let (root, state, plan) = fixture();

        assert_eq!(classify(&state, &plan), Ok(SettlementState::None));
        begin(&state, &plan).unwrap();
        begin(&state, &plan).unwrap();

        assert_eq!(classify(&state, &plan), Ok(SettlementState::Pending));
        exact_bytes(&root, QUARANTINE_TRANSITION_MARKER, &plan);
        drop(state);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn staged_receipt_resumes_to_durable_settlement_without_deleting_pending() {
        let (root, state, plan) = fixture();
        begin(&state, &plan).unwrap();
        fail_before_next_receipt_publication();

        assert_eq!(settle(&state, &plan), SettlementPublication::StillPending);
        assert_eq!(classify(&state, &plan), Ok(SettlementState::ReceiptStaged));
        exact_bytes(&root, QUARANTINE_TRANSITION_MARKER, &plan);
        exact_bytes(&root, QUARANTINE_SETTLEMENT_STAGE, &plan);

        assert_eq!(settle(&state, &plan), SettlementPublication::DurableSettled);
        assert_eq!(classify(&state, &plan), Ok(SettlementState::Settled));
        exact_bytes(&root, QUARANTINE_TRANSITION_MARKER, &plan);
        exact_bytes(&root, QUARANTINE_SETTLEMENT_RECEIPT, &plan);
        assert!(!root.join(QUARANTINE_SETTLEMENT_STAGE).exists());
        drop(state);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn visible_receipt_with_unknown_durability_is_acknowledged_on_replay() {
        let (root, state, plan) = fixture();
        begin(&state, &plan).unwrap();
        fail_after_next_receipt_rename();

        assert_eq!(
            settle(&state, &plan),
            SettlementPublication::ReceiptVisibleDurabilityUnacknowledged
        );
        assert_eq!(classify(&state, &plan), Ok(SettlementState::Settled));
        exact_bytes(&root, QUARANTINE_TRANSITION_MARKER, &plan);
        exact_bytes(&root, QUARANTINE_SETTLEMENT_RECEIPT, &plan);

        fail_next_receipt_acknowledgement();
        assert_eq!(
            settle(&state, &plan),
            SettlementPublication::ReceiptVisibleDurabilityUnacknowledged
        );
        assert_eq!(settle(&state, &plan), SettlementPublication::DurableSettled);
        drop(state);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_or_cross_plan_receipts_fail_closed() {
        let (root, state, plan) = fixture();
        begin(&state, &plan).unwrap();
        let mut different_plan = plan.clone();
        different_plan.plan_id = "sha256:different-plan".to_owned();

        assert_eq!(classify(&state, &different_plan), Err(HostFailure::Invalid));
        assert_eq!(
            settle(&state, &different_plan),
            SettlementPublication::Invalid
        );

        std::fs::write(root.join(QUARANTINE_TRANSITION_MARKER), b"not-the-plan").unwrap();

        assert_eq!(classify(&state, &plan), Err(HostFailure::Invalid));
        assert_eq!(settle(&state, &plan), SettlementPublication::Invalid);

        drop(state);
        std::fs::remove_dir_all(root).unwrap();
    }
}
