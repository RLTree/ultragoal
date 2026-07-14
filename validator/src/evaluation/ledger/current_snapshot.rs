struct CurrentSnapshot {
    snapshot: AuthenticatedSnapshot,
    partial_tail_from: Option<u64>,
    observed_state_head_sha256: String,
    observed_anchor: FileIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingPublication {
    name: String,
    generation: u64,
    identity: FileIdentity,
}
