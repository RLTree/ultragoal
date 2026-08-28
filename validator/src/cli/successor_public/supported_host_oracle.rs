pub(super) const MISSING_CAUSE: &str = "supported Codex help/listing has no proven zero-write observation surface; no action was emitted";
pub(super) const MISSING_CODE: &str = "supported-host-zero-write-oracle-unavailable";

/// No production reader may launch Codex until an independently reviewed,
/// OS-enforced zero-write help/listing observation replaces this closed gate.
pub(super) const fn is_available() -> bool {
    false
}
