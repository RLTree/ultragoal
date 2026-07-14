use super::*;

#[cfg(test)]
use std::cell::RefCell;

pub(crate) type HmacSha256 = Hmac<Sha256>;

pub(crate) const LEDGER_SCHEMA: &str = "harness-ultragoal.repository-fit-authority-ledger.v3";
pub(crate) const ENVELOPE_SCHEMA: &str =
    "harness-ultragoal.repository-fit-authority-ledger-envelope.v3";
pub(crate) const EVENT_DOMAIN: &str = "repository-fit-authority-ledger-event-v3";
pub(crate) const INITIAL_HEAD_DOMAIN: &str = "repository-fit-authority-ledger-initial-head-v3";
pub(crate) const AUTHORITY_DOMAIN: &str = "repository-fit-production-authority-v3";
pub(crate) const LOCK_MARKER: &[u8] = b"repository-fit-authority-lock-v3\n";
pub(crate) const KEY_BYTES: usize = 32;
pub(crate) const MAX_LEDGER_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_EVENTS: usize = 100_000;
pub(crate) const MAX_RECOVERY_ROWS: usize = 512;
pub(crate) const MAX_STORE_ENTRIES: usize = 4;
pub(crate) const KEY_NAME: &str = "authority.key";
pub(crate) const LOCK_NAME: &str = "authority.lock";
pub(crate) const STATE_NAME: &str = "authority-ledger.json";

#[cfg(test)]
thread_local! {
    static BEFORE_LOCK_ACQUIRE: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
    static BEFORE_ATOMIC_PUBLISH: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
    static BEFORE_EXISTING_OPEN: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

#[cfg(test)]
pub(crate) fn before_lock_acquire_for_test(action: impl FnOnce() + 'static) {
    BEFORE_LOCK_ACQUIRE.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(prior.is_none(), "a lock-acquire test hook is already armed");
    });
}

#[cfg(test)]
pub(crate) fn before_atomic_publish_for_test(action: impl FnOnce() + 'static) {
    BEFORE_ATOMIC_PUBLISH.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(
            prior.is_none(),
            "an atomic-publish test hook is already armed"
        );
    });
}

#[cfg(test)]
pub(crate) fn before_existing_open_for_test(action: impl FnOnce() + 'static) {
    BEFORE_EXISTING_OPEN.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(
            prior.is_none(),
            "an existing-open test hook is already armed"
        );
    });
}

#[cfg(test)]
pub(crate) fn test_before_lock_acquire() {
    BEFORE_LOCK_ACQUIRE.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
pub(crate) const fn test_before_lock_acquire() {}

#[cfg(test)]
pub(crate) fn test_before_atomic_publish() {
    BEFORE_ATOMIC_PUBLISH.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
pub(crate) const fn test_before_atomic_publish() {}

#[cfg(test)]
pub(crate) fn test_before_existing_open() {
    BEFORE_EXISTING_OPEN.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
pub(crate) const fn test_before_existing_open() {}

pub(crate) struct FileLedger {
    pub(crate) store: Store,
    pub(crate) key_identity: FileIdentity,
    pub(crate) lock_identity: FileIdentity,
    pub(crate) store_id: String,
    pub(crate) authority_id: String,
    pub(crate) local: Mutex<ObservedHead>,
}

#[derive(Clone, Debug)]
pub(crate) struct ObservedHead {
    pub(crate) generation: u64,
    pub(crate) head_sha256: String,
}

#[derive(Clone)]
pub(crate) struct Store {
    pub(crate) requested_root: PathBuf,
    pub(crate) canonical_root: PathBuf,
    pub(crate) directory: Arc<File>,
    pub(crate) root_identity: RootIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RootIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) mode: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) links: u64,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) mode: u32,
    pub(crate) length: u64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanoseconds: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotEnvelope {
    pub(crate) schema_version: String,
    pub(crate) payload: SnapshotPayload,
    pub(crate) hmac_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotPayload {
    pub(crate) schema_version: String,
    pub(crate) store_id: String,
    pub(crate) authority_id: String,
    pub(crate) key_id: String,
    pub(crate) root_identity: RootIdentity,
    pub(crate) lock_identity: FileIdentity,
    pub(crate) generation: u64,
    pub(crate) head_sha256: String,
    pub(crate) events: Vec<LedgerEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LedgerEvent {
    pub(crate) sequence: u64,
    pub(crate) event_id: String,
    pub(crate) prior_head_sha256: String,
    pub(crate) reservation_id: String,
    pub(crate) binding_sha256: String,
    pub(crate) semantic_effect_id: String,
    pub(crate) target_scope_id: String,
    pub(crate) permit_id: String,
    pub(crate) nonce_sha256: String,
    pub(crate) recovery_intent_sha256: String,
    pub(crate) issued_tick: u64,
    pub(crate) expires_tick: u64,
    pub(crate) recovery: RecoveryTargetSpec,
    pub(crate) state: RepositoryFitLedgerState,
    pub(crate) terminal_sha256: Option<String>,
    pub(crate) error_id: Option<AdapterErrorId>,
    pub(crate) transition_tick: u64,
    pub(crate) event_sha256: String,
}
