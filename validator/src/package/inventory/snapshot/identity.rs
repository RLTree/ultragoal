use super::PackageEntryKind;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;

fn hex(hasher: Sha256) -> String {
    format!("{:x}", hasher.finalize())
}

pub(super) fn tree(entries: &BTreeMap<String, PackageEntryKind>) -> String {
    let mut hasher = Sha256::new();
    for (path, kind) in entries {
        hasher.update(path.as_bytes());
        hasher.update([0]);
        let label = match kind {
            PackageEntryKind::Regular { single_link: true } => "regular-single-link",
            PackageEntryKind::Regular { single_link: false } => "regular-multiple-links",
            PackageEntryKind::Directory => "directory",
            PackageEntryKind::Symlink => "symlink",
            PackageEntryKind::Special => "special",
        };
        hasher.update(label.as_bytes());
        hasher.update([0]);
    }
    hex(hasher)
}

pub(super) fn dependencies(files: &BTreeMap<String, Arc<[u8]>>) -> String {
    let mut hasher = Sha256::new();
    for (path, bytes) in files {
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(format!("{:x}", Sha256::digest(bytes.as_ref())).as_bytes());
        hasher.update([0]);
    }
    hex(hasher)
}

pub(super) fn snapshot(
    context_id: &str,
    manifest: &[u8],
    tree_sha256: &str,
    dependency_sha256: &str,
    package_digest: &str,
) -> String {
    let identity = serde_json::json!({
        "schema": "PackageSnapshot-v1",
        "context_id": context_id,
        "manifest_sha256": format!("{:x}", Sha256::digest(manifest)),
        "tree_index_sha256": tree_sha256,
        "dependency_set_sha256": dependency_sha256,
        "package_digest": package_digest
    });
    crate::digest::bytes(identity.to_string().as_bytes())
}
