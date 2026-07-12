use super::evidence::{Actor, ActorRole};
use super::false_pass::{EXECUTION_METHOD, EXPECTED_EXIT_CODE, OBSERVATION_METHOD};
use super::false_pass_receipt::FalsePassExecution;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_execution_shape(execution: &FalsePassExecution) -> Result<(), String> {
    if execution.execution_id.is_empty()
        || !raw_digest(&execution.registry_digest)
        || execution.claim_id.is_empty()
        || execution.control_id.is_empty()
        || !digest(&execution.control_definition_digest)
        || !digest(&execution.command_spec_digest)
        || !digest(&execution.argument_digest)
        || !digest(&execution.script_digest)
        || !digest(&execution.authority_nonce)
        || execution.expected_failure_contract.is_empty()
        || execution.executor.actor_id.is_empty()
        || !execution
            .executor
            .actor_id
            .starts_with("claim-control-executor:")
        || execution.executor.roles != BTreeSet::from([ActorRole::ControlExecutor])
        || !execution
            .execution_method
            .starts_with(&format!("{EXECUTION_METHOD}:"))
        || execution.started_at_unix_ms == 0
        || execution.ended_at_unix_ms <= execution.started_at_unix_ms
        || execution.max_age_ms == 0
        || !digest(&execution.negative_stimulus_digest)
        || !digest(&execution.executor_tool_digest)
        || execution.exit_code != EXPECTED_EXIT_CODE
        || !digest_map(&execution.output_digests)
        || execution.artifact_digests.len() != 1
        || execution
            .artifact_digests
            .iter()
            .any(|value| !digest(value))
        || !digest(execution.actual_causal_outcome.outcome_digest())
        || !execution
            .actual_causal_outcome
            .is_expected_failure(&execution.expected_failure_contract)
    {
        return Err("claims-false-pass-execution-incomplete".to_owned());
    }
    Ok(())
}

pub(super) fn seal_digest(
    execution: &FalsePassExecution,
    observer: &Actor,
    observation_method: &str,
    observed_at_unix_ms: u64,
    captured_process_digest: &str,
) -> Result<String, String> {
    if !observer.actor_id.starts_with("claim-control-observer:") {
        return Err("claims-false-pass-observer-incomplete".to_owned());
    }
    if observer.roles
        != BTreeSet::from([ActorRole::ExecutionObserver, ActorRole::IndependentObserver])
        || !observation_method.starts_with(&format!("{OBSERVATION_METHOD}:"))
        || observed_at_unix_ms == 0
        || !digest(captured_process_digest)
        || captured_process_digest != execution.actual_causal_outcome.outcome_digest()
    {
        return Err("claims-false-pass-observer-incomplete".to_owned());
    }
    let bytes = serde_json::to_vec(&(
        execution,
        observer,
        observation_method,
        observed_at_unix_ms,
        captured_process_digest,
    ))
    .map_err(|_| "claims-false-pass-encode-failed")?;
    Ok(digest_bytes(&bytes))
}

fn digest_map(values: &BTreeMap<String, String>) -> bool {
    !values.is_empty()
        && values
            .iter()
            .all(|(key, value)| !key.is_empty() && digest(value))
}
fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
fn raw_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
