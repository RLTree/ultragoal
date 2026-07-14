fn recompute_action_id(action: &RootActionRequest) -> String {
    #[derive(Serialize)]
    struct Commitment<'a> {
        schema_version: &'a str,
        operation: RootOperation,
        reason: RootActionReason,
        authority_binding: &'a Binding,
        workspace_identity: &'a str,
        journal_head_identity: &'a str,
        expected_head: &'a JournalHead,
        snapshot_id: &'a str,
        target: &'a PermitTarget,
    }
    let bytes = serde_json::to_vec(&Commitment {
        schema_version: &action.schema_version,
        operation: action.operation,
        reason: action.reason,
        authority_binding: &action.authority_binding,
        workspace_identity: &action.workspace_identity,
        journal_head_identity: &action.journal_head_identity,
        expected_head: &action.expected_head,
        snapshot_id: &action.snapshot_id,
        target: &action.target,
    })
    .unwrap();
    format!("sha256:{:x}", Sha256::digest(bytes))
}
