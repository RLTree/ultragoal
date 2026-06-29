use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn id(prefix: &str, operation: &str, candidate: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let digest =
        crate::digest::bytes(format!("{prefix}:{operation}:{candidate}:{nanos}").as_bytes());
    format!("{prefix}-{}", digest.trim_start_matches("sha256:"))
}
