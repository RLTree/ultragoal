use super::*;

pub(crate) const STATE_COMPONENTS: &[&str] =
    &[".codex", "state", "harness-ultragoal", "routine-public"];
pub(crate) const AUTHORITY_DIRECTORY: &str = "authority";
pub(crate) const ADAPTER_DIRECTORY: &str = "adapter";
pub(crate) const LOCK_NAME: &str = "adapter.lock";
pub(crate) const LOCK_MARKER: &[u8] = b"routine-public-lock-v1\n";
pub(crate) const CACHE_NAME: &str = "reuse.json";
pub(crate) const CACHE_SCHEMA: &str = "RoutinePublicReuseCache-v2";
pub(crate) const RETIRED_CACHE_SCHEMA: &str = "RoutinePublicReuseCache-v1";
pub(crate) const MAX_CACHE_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const NONCE_BYTES: usize = 32;

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
}

pub(crate) struct HostState {
    pub(crate) home: AnchoredDirectory,
    pub(crate) state: AnchoredDirectory,
    pub(crate) authority: AnchoredDirectory,
    pub(crate) adapter: AnchoredDirectory,
    pub(crate) lock: File,
    pub(crate) lock_identity: Identity,
    pub(crate) target_id: String,
}

pub(crate) struct ProcessLock(pub(crate) File);

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CacheEnvelope {
    pub(crate) schema_version: String,
    pub(crate) binding: CacheBinding,
    pub(crate) issued_monotonic_tick: u64,
    pub(crate) nonce_hex: String,
    pub(crate) artifact_sha256: Vec<String>,
    pub(crate) artifacts_hex: Vec<String>,
}
