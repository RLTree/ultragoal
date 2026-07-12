use super::evidence::Actor;
use super::false_pass::ReceiptAuthority;
use super::false_pass_integrity::{seal_digest, validate_execution_shape};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ExecutionOutcome {
    ExecutedFailure {
        failure_code: String,
        outcome_digest: String,
    },
}

impl ExecutionOutcome {
    pub fn outcome_digest(&self) -> &str {
        let Self::ExecutedFailure { outcome_digest, .. } = self;
        outcome_digest
    }

    pub fn is_expected_failure(&self, expected_failure_contract: &str) -> bool {
        matches!(self, Self::ExecutedFailure { failure_code, .. } if failure_code == expected_failure_contract)
    }
}

impl<'de> Deserialize<'de> for ExecutionOutcome {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "claims-execution-outcome-deserialization-prohibited",
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FalsePassExecution {
    pub(super) execution_id: String,
    pub(super) registry_digest: String,
    pub(super) claim_id: String,
    pub(super) control_id: String,
    pub(super) control_definition_digest: String,
    pub(super) command_spec_digest: String,
    pub(super) argument_digest: String,
    pub(super) script_digest: String,
    pub(super) authority_nonce: String,
    pub(super) negative_stimulus_digest: String,
    pub(super) expected_failure_contract: String,
    pub(super) executor: Actor,
    pub(super) execution_method: String,
    pub(super) executor_tool_digest: String,
    pub(super) started_at_unix_ms: u64,
    pub(super) ended_at_unix_ms: u64,
    pub(super) live_context_id: String,
    pub(super) candidate_id: String,
    pub(super) max_age_ms: u64,
    pub(super) truth_surface: String,
    pub(super) declared_ceiling: String,
    pub(super) exit_code: i32,
    pub(super) actual_causal_outcome: ExecutionOutcome,
    pub(super) output_digests: BTreeMap<String, String>,
    pub(super) artifact_digests: BTreeSet<String>,
}

impl FalsePassExecution {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn from_authority(
        _authority: &ReceiptAuthority,
        execution_id: String,
        registry_digest: String,
        claim_id: String,
        control_id: String,
        control_definition_digest: String,
        command_spec_digest: String,
        argument_digest: String,
        script_digest: String,
        authority_nonce: String,
        negative_stimulus_digest: String,
        expected_failure_contract: String,
        executor: Actor,
        execution_method: String,
        executor_tool_digest: String,
        started_at_unix_ms: u64,
        ended_at_unix_ms: u64,
        live_context_id: String,
        candidate_id: String,
        max_age_ms: u64,
        truth_surface: String,
        declared_ceiling: String,
        exit_code: i32,
        actual_causal_outcome: ExecutionOutcome,
        output_digests: BTreeMap<String, String>,
        artifact_digests: BTreeSet<String>,
    ) -> Self {
        Self {
            execution_id,
            registry_digest,
            claim_id,
            control_id,
            control_definition_digest,
            command_spec_digest,
            argument_digest,
            script_digest,
            authority_nonce,
            negative_stimulus_digest,
            expected_failure_contract,
            executor,
            execution_method,
            executor_tool_digest,
            started_at_unix_ms,
            ended_at_unix_ms,
            live_context_id,
            candidate_id,
            max_age_ms,
            truth_surface,
            declared_ceiling,
            exit_code,
            actual_causal_outcome,
            output_digests,
            artifact_digests,
        }
    }

    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }
    pub fn registry_digest(&self) -> &str {
        &self.registry_digest
    }
    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }
    pub fn control_id(&self) -> &str {
        &self.control_id
    }
    pub fn control_definition_digest(&self) -> &str {
        &self.control_definition_digest
    }
    pub fn command_spec_digest(&self) -> &str {
        &self.command_spec_digest
    }
    pub fn argument_digest(&self) -> &str {
        &self.argument_digest
    }
    pub fn script_digest(&self) -> &str {
        &self.script_digest
    }
    pub fn authority_nonce(&self) -> &str {
        &self.authority_nonce
    }
    pub fn negative_stimulus_digest(&self) -> &str {
        &self.negative_stimulus_digest
    }
    pub fn expected_failure_contract(&self) -> &str {
        &self.expected_failure_contract
    }
    pub fn executor(&self) -> &Actor {
        &self.executor
    }
    pub fn execution_method(&self) -> &str {
        &self.execution_method
    }
    pub fn executor_tool_digest(&self) -> &str {
        &self.executor_tool_digest
    }
    pub fn started_at_unix_ms(&self) -> u64 {
        self.started_at_unix_ms
    }
    pub fn ended_at_unix_ms(&self) -> u64 {
        self.ended_at_unix_ms
    }
    pub fn live_context_id(&self) -> &str {
        &self.live_context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn max_age_ms(&self) -> u64 {
        self.max_age_ms
    }
    pub fn truth_surface(&self) -> &str {
        &self.truth_surface
    }
    pub fn declared_ceiling(&self) -> &str {
        &self.declared_ceiling
    }
    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }
    pub fn actual_causal_outcome(&self) -> &ExecutionOutcome {
        &self.actual_causal_outcome
    }
    pub fn output_digests(&self) -> &BTreeMap<String, String> {
        &self.output_digests
    }
    pub fn artifact_digests(&self) -> &BTreeSet<String> {
        &self.artifact_digests
    }
}

impl<'de> Deserialize<'de> for FalsePassExecution {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "claims-false-pass-execution-deserialization-prohibited",
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExecutionObservation {
    execution: FalsePassExecution,
    observer: Actor,
    observation_method: String,
    observed_at_unix_ms: u64,
    captured_process_digest: String,
    observed_digest: String,
}

impl ExecutionObservation {
    pub(super) fn from_authority(
        _authority: &ReceiptAuthority,
        execution: FalsePassExecution,
        observer: Actor,
        observation_method: String,
        observed_at_unix_ms: u64,
        captured_process_digest: String,
    ) -> Result<Self, String> {
        let observed_digest = seal_digest(
            &execution,
            &observer,
            &observation_method,
            observed_at_unix_ms,
            &captured_process_digest,
        )?;
        Ok(Self {
            execution,
            observer,
            observation_method,
            observed_at_unix_ms,
            captured_process_digest,
            observed_digest,
        })
    }

    pub fn execution(&self) -> &FalsePassExecution {
        &self.execution
    }
    pub fn observer(&self) -> &Actor {
        &self.observer
    }
    pub fn observation_method(&self) -> &str {
        &self.observation_method
    }
    pub fn observed_at_unix_ms(&self) -> u64 {
        self.observed_at_unix_ms
    }
    pub fn observed_digest(&self) -> &str {
        &self.observed_digest
    }

    pub fn verify_integrity(&self) -> Result<(), String> {
        validate_execution_shape(&self.execution)?;
        let actual = seal_digest(
            &self.execution,
            &self.observer,
            &self.observation_method,
            self.observed_at_unix_ms,
            &self.captured_process_digest,
        )?;
        if actual == self.observed_digest {
            Ok(())
        } else {
            Err("claims-false-pass-observation-mutated".to_owned())
        }
    }
}

impl<'de> Deserialize<'de> for ExecutionObservation {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "claims-false-pass-observation-deserialization-prohibited",
        ))
    }
}
