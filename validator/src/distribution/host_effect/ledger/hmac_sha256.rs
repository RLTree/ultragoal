type HmacSha256 = Hmac<Sha256>;

const LEDGER_SCHEMA: &str = "harness-ultragoal.host-effect-ledger.v2";
const EVENT_SCHEMA: &str = "harness-ultragoal.host-effect-ledger-event.v2";
const INITIAL_HEAD_SCHEMA: &str = "harness-ultragoal.host-effect-ledger-initial-head.v2";
const KEY_BYTES: usize = 32;
const MAX_LEDGER_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EVENTS: usize = 100_000;
const KEY_NAME: &str = "ledger.key";
const LOCK_NAME: &str = "ledger.lock";
const STATE_NAME: &str = "ledger.json";

/// Durable, cross-process, fail-closed implementation of the host-effect
/// ledger interface. The directory and key object are descriptor-bound at
/// construction. Every mutation publishes one authenticated, hash-chained
/// snapshot through an atomic rename and fsyncs the containing directory.
pub(crate) struct FileHostEffectLedger {
    root: PathBuf,
    canonical_root: PathBuf,
    directory: Arc<File>,
    directory_identity: FileIdentity,
    key_identity: FileIdentity,
    lock_identity: FileIdentity,
    ledger_id: String,
    local: Mutex<ObservedHead>,
    #[cfg(test)]
    lock_open_hook: Mutex<Option<LockOpenHook>>,
    #[cfg(test)]
    key_open_hook: Mutex<Option<KeyOpenHook>>,
}

#[cfg(test)]
#[derive(Clone)]
struct LockOpenHook {
    reached: Arc<std::sync::Barrier>,
    release: Arc<std::sync::Barrier>,
}

#[cfg(test)]
#[derive(Clone)]
struct KeyOpenHook {
    reached: Arc<std::sync::Barrier>,
    release: Arc<std::sync::Barrier>,
}

#[derive(Clone, Debug)]
struct ObservedHead {
    initialized: bool,
    generation: u64,
    head_sha256: String,
}
