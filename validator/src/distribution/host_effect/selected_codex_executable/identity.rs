use crate::distribution::host_effect::{HostEffectLedgerError, HostEffectLedgerErrorId};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(in crate::distribution::host_effect) struct SelectedCodexExecutableIdentity {
    pub(in crate::distribution::host_effect) canonical_path: String,
    pub(super) content_sha256: String,
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) mode: u32,
    pub(super) uid: u32,
    pub(super) gid: u32,
    pub(super) hard_links: u64,
    pub(super) size: u64,
    pub(super) modified_seconds: i64,
    pub(super) modified_nanoseconds: i64,
    pub(super) changed_seconds: i64,
    pub(super) changed_nanoseconds: i64,
}

impl SelectedCodexExecutableIdentity {
    pub(in crate::distribution::host_effect) fn binding_sha256(
        &self,
    ) -> Result<String, HostEffectLedgerError> {
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            identity: &'a SelectedCodexExecutableIdentity,
        }
        serde_json::to_vec(&Binding {
            schema: "harness-ultragoal.pinned-host-executable.v1",
            identity: self,
        })
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
        .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord))
    }
}
