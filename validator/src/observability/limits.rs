pub(super) const EVENT_SCHEMA: &str = "SemanticEvent-v1";
pub(super) const ROW_SCHEMA: &str = "SemanticEventRow-v1";

pub(super) const MAX_IDENTIFIER_BYTES: usize = 128;
pub(super) const MAX_ATTRIBUTE_KEY_BYTES: usize = 64;
pub(super) const MAX_ATTRIBUTE_VALUE_BYTES: usize = 512;
pub(super) const MAX_ATTRIBUTES: usize = 32;
pub(super) const MAX_REFERENCES: usize = 32;
pub(super) const MAX_ROW_BYTES: usize = 8 * 1024;

pub(super) const HARD_MAX_STORE_BYTES: u64 = 4 * 1024 * 1024;
pub(super) const HARD_MAX_EVENTS: usize = 4096;
pub(super) const HARD_MAX_SCAN_ROWS: usize = 4096;
pub(super) const HARD_MAX_RESULTS: usize = 256;
pub(super) const DEFAULT_RESULTS: usize = 100;
pub(super) const MAX_DURATION_MS: u64 = 24 * 60 * 60 * 1000;
