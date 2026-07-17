/// Process-local mutation observation. This is deliberately neither public nor
/// serializable and is never accepted as fresh-process custody authority.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct LedgerObservation {
    pub(crate) sequence: u64,
    pub(crate) record_id: Option<String>,
    pub(crate) byte_length: u64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanoseconds: i64,
}
