use super::*;

pub(crate) fn root_id(context_id: &str, role: &str, absolute: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"repository-fit-root-id-v1\0");
    hasher.update(context_id.as_bytes());
    hasher.update([0]);
    hasher.update(role.as_bytes());
    hasher.update([0]);
    hasher.update(absolute.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}
