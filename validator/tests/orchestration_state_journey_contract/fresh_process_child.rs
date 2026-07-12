use super::support::*;
use crate::orchestration::product::command::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

pub const CHILD_ENV: &str = "HUL_ORCHESTRATION_JOURNEY_INPUT";
pub const CHILD_TEST: &str =
    "orchestration_state_journey_contract::fresh_process_child::fresh_process_child";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildInput {
    pub mode: String,
    pub journal_root: PathBuf,
    pub output: PathBuf,
    pub expected_head: JournalHead,
    pub expected_event_id: Option<String>,
    pub recovered_binding: Option<Binding>,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildOutput {
    pub status: String,
    pub error_code: Option<String>,
    pub workspace_identity: Option<String>,
    pub current_head: Option<JournalHead>,
    pub snapshot: Option<ProductSnapshot>,
    pub action: Option<RootActionRequest>,
}

#[test]
fn fresh_process_child() {
    let Some(input_path) = std::env::var_os(CHILD_ENV) else {
        return;
    };
    let input: ChildInput =
        serde_json::from_slice(&fs::read(PathBuf::from(input_path)).unwrap()).unwrap();
    let output = match execute(&input) {
        Ok(output) => output,
        Err(error) => ChildOutput {
            status: "refused".to_owned(),
            error_code: Some(error.code().to_owned()),
            workspace_identity: None,
            current_head: None,
            snapshot: None,
            action: None,
        },
    };
    fs::write(&input.output, serde_json::to_vec(&output).unwrap()).unwrap();
}

fn execute(input: &ChildInput) -> Result<ChildOutput, ProductError> {
    let workspace = ProductWorkspace::open(&input.journal_root)?;
    match input.mode.as_str() {
        "inspect" => inspect_state(input, &workspace),
        "resume" => resume_state(input, &workspace),
        "reconcile" => reconcile_state(input, &workspace),
        "recover" => recover_state(input, &workspace),
        _ => Err(ProductError::AuthorityOperationMismatch),
    }
}

fn inspect_state(
    input: &ChildInput,
    workspace: &ProductWorkspace,
) -> Result<ChildOutput, ProductError> {
    let view = inspect(
        &context(),
        workspace,
        &OrchestrationStateRequest {
            expected_head: input.expected_head.clone(),
            tick: input.tick,
            live_workers: input.live_workers.clone(),
        },
    )?;
    let action = next(&view).root_action_request;
    Ok(ChildOutput {
        status: "inspected".to_owned(),
        error_code: None,
        workspace_identity: Some(workspace.identity().to_owned()),
        current_head: Some(view.snapshot.journal_head.clone()),
        snapshot: Some(view.snapshot),
        action,
    })
}

fn resume_state(
    input: &ChildInput,
    workspace: &ProductWorkspace,
) -> Result<ChildOutput, ProductError> {
    let view = inspect(
        &context(),
        workspace,
        &OrchestrationStateRequest {
            expected_head: input.expected_head.clone(),
            tick: input.tick,
            live_workers: input.live_workers.clone(),
        },
    )?;
    let action = next(&view)
        .root_action_request
        .ok_or(ProductError::AuthorityInvalid)?;
    view.validate_action(workspace, &action)?;
    let (authority, permit) = permit_for_action(&action, input.tick);
    let outcome = resume(
        &context(),
        workspace,
        &authority,
        &permit,
        &ResumeRequest {
            expected_head: action.expected_head.clone(),
            tick: input.tick,
            live_workers: input.live_workers.clone(),
            target: action.target.clone(),
        },
    )?;
    Ok(ChildOutput {
        status: "resumed".to_owned(),
        error_code: None,
        workspace_identity: Some(workspace.identity().to_owned()),
        current_head: Some(outcome.current_head),
        snapshot: Some(outcome.snapshot),
        action: Some(action),
    })
}

fn reconcile_state(
    input: &ChildInput,
    workspace: &ProductWorkspace,
) -> Result<ChildOutput, ProductError> {
    let view = inspect(
        &context(),
        workspace,
        &OrchestrationStateRequest {
            expected_head: input.expected_head.clone(),
            tick: input.tick,
            live_workers: input.live_workers.clone(),
        },
    )?;
    let action = next(&view)
        .root_action_request
        .ok_or(ProductError::AuthorityInvalid)?;
    view.validate_action(workspace, &action)?;
    let lease_id = action
        .target
        .lease_id
        .clone()
        .ok_or(ProductError::UnknownWorker)?;
    let operation_id = action
        .target
        .operation_id
        .clone()
        .ok_or(ProductError::UnknownOperation)?;
    let resolution = EffectResolution {
        operation_id,
        evidence_digest: digest('e'),
        outcome: EffectOutcome::NotApplied,
    };
    let (authority, permit) = permit_for_reconciliation(&action, input.tick, &resolution);
    let outcome = reconcile(
        &context(),
        workspace,
        &authority,
        &permit,
        &ReconcileRequest {
            expected_head: action.expected_head.clone(),
            tick: input.tick,
            live_workers: input.live_workers.clone(),
            lease_id,
            resolution,
            target: action.target.clone(),
        },
    )?;
    Ok(ChildOutput {
        status: "reconciled".to_owned(),
        error_code: None,
        workspace_identity: Some(workspace.identity().to_owned()),
        current_head: Some(outcome.current_head),
        snapshot: None,
        action: Some(action),
    })
}

fn recover_state(
    input: &ChildInput,
    workspace: &ProductWorkspace,
) -> Result<ChildOutput, ProductError> {
    let expected_event_id = input
        .expected_event_id
        .clone()
        .ok_or(ProductError::UnknownOperation)?;
    let recovered_binding = input
        .recovered_binding
        .clone()
        .ok_or(ProductError::StaleCandidate)?;
    let preview = inspect_interrupted(
        &context(),
        workspace,
        &InterruptedRecoveryRequest {
            expected_prior_head: input.expected_head.clone(),
            expected_event_id: expected_event_id.clone(),
            recovered_binding: recovered_binding.clone(),
            tick: input.tick,
            live_workers: input.live_workers.clone(),
        },
    )?;
    let action = preview.root_action_request.clone();
    preview.validate_action(workspace, &action)?;
    let (authority, permit) = permit_for_action(&action, input.tick);
    let outcome = recover(
        &context(),
        workspace,
        &authority,
        &permit,
        &RecoverRequest {
            expected_prior_head: input.expected_head.clone(),
            expected_event_id,
            recovered_binding,
            tick: input.tick,
            live_workers: input.live_workers.clone(),
            target: action.target.clone(),
        },
    )?;
    Ok(ChildOutput {
        status: "recovered".to_owned(),
        error_code: None,
        workspace_identity: Some(outcome.workspace_identity),
        current_head: Some(outcome.recovered_head),
        snapshot: Some(outcome.snapshot),
        action: Some(action),
    })
}
