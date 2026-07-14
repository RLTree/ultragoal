use super::*;

pub(crate) fn terminal_digest(
    request_id: &str,
    state: RepositoryFitLedgerState,
    error_id: Option<AdapterErrorId>,
    effect_started: bool,
    rollback_complete: bool,
    outcome_id: Option<&str>,
) -> String {
    digest(
        &serde_json::to_vec(&(
            "repository-fit-production-terminal-v1",
            request_id,
            state,
            error_id,
            effect_started,
            rollback_complete,
            outcome_id,
        ))
        .expect("fixed terminal projection is serializable"),
    )
}
