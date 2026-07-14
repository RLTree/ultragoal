use super::*;

#[derive(Clone, Debug)]
pub struct EventStore {
    pub(crate) path: PathBuf,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) source_id: String,
    pub(crate) max_store_bytes: u64,
    pub(crate) max_events: usize,
    pub(crate) max_scan_rows: usize,
    pub(crate) max_results: usize,
    pub(in crate::observability) identity: BoundStoreIdentity,
}
