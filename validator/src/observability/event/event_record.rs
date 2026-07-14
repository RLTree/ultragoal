use super::*;

/// A bounded observation. It deliberately has no claim-state field or claim mutation API.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticEvent {
    pub(crate) schema_version: String,
    pub(crate) event_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) source_id: String,
    pub(crate) observed_at_unix_ms: u64,
    pub(crate) sequence: u64,
    pub(crate) parent_event_id: Option<String>,
    pub(crate) operation: String,
    pub(crate) outcome: String,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) selected_work: Option<u64>,
    pub(crate) reused_work: Option<u64>,
    pub(crate) skipped_work: Option<u64>,
    pub(crate) public_attributes: BTreeMap<String, String>,
    pub(crate) redacted_attribute_keys: BTreeSet<String>,
    pub(crate) finding_refs: BTreeSet<String>,
    pub(crate) repair_refs: BTreeSet<String>,
}
