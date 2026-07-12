use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(test)]
use crate::capture::{ArtifactDisposition, ArtifactResolver, CapturedArtifact, CapturedRun};
#[cfg(not(test))]
use crate::capture::{ArtifactDisposition, ArtifactResolver, CapturedArtifact, CapturedRun};
use crate::context::LiveContext;

use super::model::{EvidenceBinding, ReuseExpectation, RunOutcome, valid_result_map};
use super::receipt;
use super::witness::{CapturedExecution, ExecutedWork, ObservedResult, ResultFacts};
#[cfg(test)]
use crate::routine_work::authority::test_authority_checkpoint;
use crate::routine_work::authority::{current_binding, ensure_unchanged};
use crate::routine_work::digest::{canonical, sha256, valid};
use crate::routine_work::{RoutineError, RoutineErrorId};

const MAX_RESULT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct CaptureRunWire {
    schema_version: String,
    tool_id: String,
    command_id: String,
    authority_context_id: String,
    observed_context_id: String,
    candidate_before: CandidateWire,
    candidate_after: CandidateWire,
    effect: String,
    invocation_metadata_disposition: String,
    program_path_hex: String,
    program_sha256: String,
    enforcement: EnforcementWire,
    termination: TerminationWire,
}

#[derive(Debug, Deserialize)]
struct CandidateWire {
    candidate_id: String,
}

#[derive(Debug, Deserialize)]
struct EnforcementWire {
    network_allowed: bool,
    process_fork_allowed: bool,
    external_write_allowed: bool,
    execution_allowed: bool,
    preflight_passed: bool,
}

#[derive(Debug, Deserialize)]
struct TerminationWire {
    kind: String,
    code: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResultArtifactWire {
    schema_version: String,
    binding: EvidenceBinding,
    capture_command_id: String,
    capture_program_sha256: String,
    outcome: RunOutcome,
    behavior_observed: bool,
    behavior_sha256: String,
    output_digests: BTreeMap<String, String>,
}

struct ParsedResult {
    facts: ResultFacts,
    capture_command_id: String,
    capture_program_sha256: String,
}

pub fn capture_executed_result(
    context: &LiveContext,
    expectation: &ReuseExpectation,
    run: &CapturedRun,
) -> Result<CapturedExecution, RoutineError> {
    let current = current_binding(context)?;
    if expectation.binding.live_miss(&current).is_some() {
        return Err(authority("executed-capture-live-binding-mismatch"));
    }
    #[cfg(test)]
    test_authority_checkpoint();
    let run_json = run
        .to_canonical_json()
        .map_err(|_| observation("capture-run-serialization-failed"))?;
    let authority: CaptureRunWire = serde_json::from_slice(&run_json)
        .map_err(|_| observation("capture-run-envelope-invalid"))?;
    validate_run(expectation, &authority)?;
    let [artifact] = run.artifacts() else {
        return Err(observation("captured-run-result-artifact-count-inexact"));
    };
    let parsed = parse_result(expectation, artifact)?;
    if authority.command_id != parsed.capture_command_id
        || authority.program_sha256 != parsed.capture_program_sha256
    {
        return Err(observation("captured-run-result-tool-binding-mismatch"));
    }
    let work = ExecutedWork {
        facts: parsed.facts,
        capture_run_sha256: sha256(&run_json),
    };
    let receipt_json = receipt::completed_bytes(expectation, &work)?;
    ensure_unchanged(
        context,
        &current,
        "routine-binding-changed-during-executed-capture",
    )?;
    Ok(CapturedExecution::new(work, receipt_json))
}

pub fn observe_result_artifact(
    context: &LiveContext,
    expectation: &ReuseExpectation,
    artifact: &CapturedArtifact,
) -> Result<ObservedResult, RoutineError> {
    let current = current_binding(context)?;
    if expectation.binding.live_miss(&current).is_some() {
        return Err(authority("result-observation-live-binding-mismatch"));
    }
    #[cfg(test)]
    test_authority_checkpoint();
    let observed = parse_result(expectation, artifact).map(|parsed| ObservedResult {
        facts: parsed.facts,
    })?;
    ensure_unchanged(
        context,
        &current,
        "routine-binding-changed-during-result-observation",
    )?;
    Ok(observed)
}

fn parse_result(
    expectation: &ReuseExpectation,
    artifact: &CapturedArtifact,
) -> Result<ParsedResult, RoutineError> {
    if artifact.disposition() != ArtifactDisposition::Public
        || artifact.context_id() != expectation.binding.context_id
        || artifact.candidate_id() != expectation.binding.candidate_id
        || artifact.byte_length() as usize > MAX_RESULT_BYTES
    {
        return Err(observation("result-artifact-authority-binding-invalid"));
    }
    let reference = artifact.artifact_ref();
    let bytes = artifact
        .resolve(&reference)
        .map_err(|_| observation("result-artifact-resolution-failed"))?;
    let wire = parse_wire(&bytes)?;
    if wire.binding != expectation.binding || wire.capture_command_id != wire.binding.node_id {
        return Err(observation("result-artifact-expectation-mismatch"));
    }
    Ok(ParsedResult {
        facts: ResultFacts {
            binding: wire.binding,
            outcome: wire.outcome,
            behavior_observed: wire.behavior_observed,
            behavior_sha256: wire.behavior_sha256,
            output_digests: wire.output_digests,
            result_artifact_sha256: reference.sha256().to_owned(),
        },
        capture_command_id: wire.capture_command_id,
        capture_program_sha256: wire.capture_program_sha256,
    })
}

fn parse_wire(bytes: &[u8]) -> Result<ResultArtifactWire, RoutineError> {
    if bytes.len() > MAX_RESULT_BYTES {
        return Err(observation("result-artifact-size-limit-exceeded"));
    }
    let wire: ResultArtifactWire =
        serde_json::from_slice(bytes).map_err(|_| observation("result-artifact-json-invalid"))?;
    if canonical(&wire)? != bytes
        || wire.schema_version != "RoutineResultArtifact-v1"
        || !valid(&wire.capture_program_sha256)
        || !valid(&wire.behavior_sha256)
        || !valid_result_map(&wire.output_digests)
    {
        return Err(observation("result-artifact-fields-invalid"));
    }
    Ok(wire)
}

fn validate_run(expectation: &ReuseExpectation, run: &CaptureRunWire) -> Result<(), RoutineError> {
    let binding = &expectation.binding;
    let enforced = run.enforcement.execution_allowed
        && run.enforcement.preflight_passed
        && !run.enforcement.network_allowed
        && !run.enforcement.process_fork_allowed
        && !run.enforcement.external_write_allowed;
    if run.schema_version != "CapturedRun-v1"
        || run.tool_id != "HCT-CAPTURE"
        || run.command_id != binding.node_id
        || run.authority_context_id != binding.context_id
        || run.observed_context_id != binding.context_id
        || run.candidate_before.candidate_id != binding.candidate_id
        || run.candidate_after.candidate_id != binding.candidate_id
        || run.effect != "read"
        || run.invocation_metadata_disposition != "public"
        || run.program_path_hex != binding.tool_program_path_hex
        || !valid(&run.program_sha256)
        || run.termination.kind != "exited"
        || run.termination.code != Some(0)
        || !enforced
    {
        return Err(observation("capture-run-authority-binding-invalid"));
    }
    Ok(())
}

#[cfg(test)]
impl ReuseExpectation {
    pub(crate) fn result_artifact_fixture(
        &self,
        capture_program_sha256: String,
        outcome: RunOutcome,
        behavior_observed: bool,
        behavior_sha256: String,
        output_digests: BTreeMap<String, String>,
    ) -> Vec<u8> {
        canonical(&ResultArtifactWire {
            schema_version: "RoutineResultArtifact-v1".to_owned(),
            binding: self.binding.clone(),
            capture_command_id: self.node_id().to_owned(),
            capture_program_sha256,
            outcome,
            behavior_observed,
            behavior_sha256,
            output_digests,
        })
        .expect("result artifact fixture")
    }
}

fn observation(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::ObservationFailed, cause, None)
}

fn authority(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::ContextMismatch, cause, None)
}
