use super::*;

pub(crate) const STATE_COMPONENTS: &[&str] =
    &[".codex", "state", "harness-ultragoal", "routine-public"];
pub(crate) const AUTHORITY_DIRECTORY: &str = "authority";
pub(crate) const ADAPTER_DIRECTORY: &str = "adapter";
pub(crate) const STATE_FORMAT_NAME: &str = "routine-state-format";
pub(crate) const STATE_FORMAT_BYTES: &[u8] = b"routine-host-state-v8\n";
pub(crate) const LOCK_NAME: &str = "adapter.lock";
pub(crate) const CONTINUITY_CHECKPOINT_NAME: &str = "routine-continuation.json";
pub(crate) const CONTINUITY_CHECKPOINT_STAGE_NAME: &str = ".routine-continuation.next";
pub(crate) const CONTINUITY_DIRECTORY_NAME: &str = "continuations";
pub(crate) const LOCK_MARKER: &[u8] = b"routine-public-lock-v1\n";
pub(crate) const BOOTSTRAP_STAGE: &str = ".routine-public-bootstrap";
pub(crate) const LAUNCH_DIRECTORY: &str = ".routine-authority-launch";
pub(crate) const EVENT_FILE_PREFIX: &str = "routine-events-";
pub(crate) const EVENT_FILE_SUFFIX: &str = ".jsonl";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DirectorySecurity {
    HostAncestry,
    PrivateAuthority,
}

impl DirectorySecurity {
    pub(crate) fn accepts(self, mode: u32) -> bool {
        mode & 0o022 == 0 && (self == Self::HostAncestry || mode & 0o7777 == 0o700)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Identity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) owner: u32,
    pub(crate) mode: u32,
    pub(crate) links: u64,
}

pub(crate) struct AnchoredDirectory {
    pub(crate) path: PathBuf,
    pub(crate) file: File,
    pub(crate) identity: Identity,
    pub(crate) security: DirectorySecurity,
}

pub(crate) struct HostState {
    pub(crate) home: AnchoredDirectory,
    pub(crate) state: AnchoredDirectory,
    pub(crate) authority: AnchoredDirectory,
    pub(crate) adapter: AnchoredDirectory,
    pub(crate) launch: AnchoredDirectory,
    pub(crate) lock: File,
    pub(crate) lock_identity: Identity,
}

pub(crate) struct HostEventStore {
    pub(crate) store: crate::observability::EventStore,
    pub(crate) leaf: String,
    pub(crate) identity: Identity,
}

pub(crate) struct ProcessLock(pub(crate) File);
