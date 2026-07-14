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
}
