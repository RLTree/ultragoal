#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InstalledPostimage {
    pub(super) root_id: String,
    pub(super) target_id: String,
    pub(super) object_sha256: String,
    pub(super) object_identity_sha256: String,
    pub(super) byte_length: u64,
    pub(super) mode: u32,
}

impl InstalledPostimage {
    pub(crate) fn new(
        root_id: String,
        target: &str,
        object_sha256: String,
        object_identity_sha256: String,
        byte_length: u64,
        mode: u32,
    ) -> Self {
        Self {
            root_id,
            target_id: sha256(target.as_bytes()),
            object_sha256,
            object_identity_sha256,
            byte_length,
            mode,
        }
    }

    pub(crate) fn same_location(&self, other: &Self) -> bool {
        self.root_id == other.root_id && self.target_id == other.target_id
    }

    pub(crate) fn matches_target(&self, root_id: &str, target: &str) -> bool {
        self.root_id == root_id && self.target_id == sha256(target.as_bytes())
    }

    pub(crate) fn object_sha256(&self) -> &str {
        &self.object_sha256
    }
}
