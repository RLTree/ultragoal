use super::super::{EXTERNAL_PROCESS_EXIT_BEHAVIOR, RUST_SOURCE_SYNTAX_BEHAVIOR};
use super::*;

pub(crate) fn mediate_intent(
    context: &LiveContext,
    plan: &RoutinePlan,
    token: &RoutineMediatedIntent,
    snapshot_id: &str,
    dependencies: &BTreeMap<String, String>,
    reuse: Option<&Vec<u8>>,
    cancellation: &RoutineCancellation,
    attempt: &AttemptReservation,
) -> Result<IntentResult, RoutineError> {
    token.require_current()?;
    validate_intent(context, plan, token.intent())?;
    let root = RootAnchor::open(context.worktree_root())?;
    let program = PinnedExecutable::open_bound(
        token.intent().program_path_hex(),
        token.intent().program_sha256(),
        token.intent().program_byte_length(),
        token.intent().program_unix_mode(),
    )?;
    let outputs = OutputConfinement::prepare(
        &root,
        token.intent().declared_output_scopes(),
        token.intent().output_budget_bytes(),
    )?;
    let reads = ReadConfinement::open_bound(&root, token.intent().read_sources())?;
    if let Some(bytes) = reuse {
        if let Some(verified) = verify_reuse_artifact(
            bytes,
            context,
            plan,
            token,
            snapshot_id,
            dependencies,
            &outputs,
            attempt,
        )? {
            reads.validate(&root)?;
            return Ok(IntentResult::Reused(verified));
        }
        if attempt
            .durable
            .as_ref()
            .is_some_and(|durable| durable.reuse_only())
        {
            return Err(mediator_error(
                "mediator-production-reuse-not-authenticated",
            ));
        }
    } else if attempt
        .durable
        .as_ref()
        .is_some_and(|durable| durable.reuse_only())
    {
        return Err(mediator_error("mediator-production-reuse-artifact-missing"));
    }
    let environment = execution_environment(token)?;
    let framed_input = if token.intent().behavior_id() == RUST_SOURCE_SYNTAX_BEHAVIOR {
        Some(reads.rust_source_syntax_frame(&root)?)
    } else if token.intent().behavior_id() == EXTERNAL_PROCESS_EXIT_BEHAVIOR {
        None
    } else {
        return Err(mediator_error("mediator-behavior-unsupported"));
    };
    let expected_behavior_stdout = framed_input
        .as_deref()
        .map(expected_rust_source_syntax_stdout)
        .transpose()?;
    let framed_input_sha256 = framed_input.as_deref().map(sha256);
    attempt.prepare_spawn()?;
    let observation = process::execute(
        &program,
        &root,
        &outputs,
        &reads,
        token.intent().argv(),
        &environment,
        framed_input,
        Duration::from_millis(token.intent().timeout_ms()),
        token.intent().output_budget_bytes(),
        cancellation,
        || attempt.mark_started(),
    );
    let observation = match observation {
        Ok(value) => value,
        Err(error) => {
            return Err(error);
        }
    };
    if observation.started {
        run_test_post_spawn_hook();
    }
    reads.validate(&root)?;
    let disposition = match observation.termination {
        ProcessTermination::Exited(0) => None,
        ProcessTermination::Cancelled => {
            Some((RoutineNodeDisposition::Cancelled, "MEDIATOR-CANCELLED"))
        }
        ProcessTermination::TimedOut => Some((RoutineNodeDisposition::Failed, "MEDIATOR-TIMEOUT")),
        ProcessTermination::OutputLimit => {
            Some((RoutineNodeDisposition::Failed, "MEDIATOR-OUTPUT-LIMIT"))
        }
        ProcessTermination::DescendantSurvived => Some((
            RoutineNodeDisposition::Failed,
            "MEDIATOR-DESCENDANT-SURVIVED",
        )),
        ProcessTermination::CleanupFailed => {
            Some((RoutineNodeDisposition::Failed, "MEDIATOR-CLEANUP-FAILED"))
        }
        ProcessTermination::Exited(_) | ProcessTermination::Signaled(_) => {
            Some((RoutineNodeDisposition::Failed, "MEDIATOR-CHECK-FAILED"))
        }
    };
    if let Some((disposition, failure_code)) = disposition {
        return Ok(IntentResult::Incomplete {
            disposition,
            failure_code,
            started: observation.started,
        });
    }
    if expected_behavior_stdout.as_ref().is_some_and(|expected| {
        observation.stdout != *expected || observation.stderr_sha256 != sha256(&[])
    }) {
        return Ok(IntentResult::Incomplete {
            disposition: RoutineNodeDisposition::Failed,
            failure_code: "MEDIATOR-BEHAVIOR-OBSERVATION-INVALID",
            started: true,
        });
    }
    let output_files = outputs.capture()?;
    let artifact_bytes = output_files
        .values()
        .try_fold(0_u64, |total, file| total.checked_add(file.byte_length));
    if artifact_bytes
        .and_then(|total| total.checked_add(observation.output_byte_length))
        .is_none_or(|total| total > token.intent().output_budget_bytes())
    {
        return Ok(IntentResult::Incomplete {
            disposition: RoutineNodeDisposition::Failed,
            failure_code: "MEDIATOR-OUTPUT-LIMIT",
            started: true,
        });
    }
    outputs.validate()?;
    reads.validate(&root)?;
    root.validate()?;
    program.validate()?;
    context
        .revalidate()
        .map_err(|_| concurrent("mediator-context-mutated-by-process"))?;
    validate_snapshot(context, snapshot_id)?;
    let behavior_sha256 = framed(&[
        RESULT_DOMAIN,
        token.intent().behavior_id().as_bytes(),
        framed_input_sha256.as_deref().unwrap_or("none").as_bytes(),
        &observation.stdout,
        observation.stderr_sha256.as_bytes(),
        digest_of(&output_files)?.as_bytes(),
    ]);
    let result = ResultArtifactWire {
        schema_version: "RoutineMediatedResultArtifact-v2".to_owned(),
        request_id: token.request_id().to_owned(),
        protocol_id: token.protocol_id().to_owned(),
        intent_id: token.intent().intent_id().to_owned(),
        node_id: token.intent().node_id().to_owned(),
        behavior_id: token.intent().behavior_id().to_owned(),
        plan_order: token.intent().plan_order(),
        context_id: context.context_id().to_owned(),
        candidate_id: plan.binding().candidate_id().to_owned(),
        plan_id: plan.plan_id().to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        input_id: token.intent().input_id().to_owned(),
        tool_identity_sha256: token.intent().tool_identity_sha256().to_owned(),
        program_sha256: token.intent().program_sha256().to_owned(),
        environment_sha256: token.intent().environment_sha256().to_owned(),
        read_authority_sha256: token.intent().read_authority_sha256().to_owned(),
        dependency_results: dependencies.clone(),
        behavior_sha256,
        output_files: output_files.clone(),
    };
    let result_bytes = canonical(&result)?;
    let result_sha256 = sha256(&result_bytes);
    let mut reuse = ReuseArtifactWire {
        schema_version: "RoutineMediatedReuseArtifact-v2".to_owned(),
        state: "complete".to_owned(),
        protocol_id: token.protocol_id().to_owned(),
        intent_id: token.intent().intent_id().to_owned(),
        node_id: token.intent().node_id().to_owned(),
        behavior_id: token.intent().behavior_id().to_owned(),
        plan_order: token.intent().plan_order(),
        context_id: context.context_id().to_owned(),
        candidate_id: plan.binding().candidate_id().to_owned(),
        plan_id: plan.plan_id().to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        input_id: token.intent().input_id().to_owned(),
        tool_identity_sha256: token.intent().tool_identity_sha256().to_owned(),
        program_sha256: token.intent().program_sha256().to_owned(),
        environment_sha256: token.intent().environment_sha256().to_owned(),
        read_authority_sha256: token.intent().read_authority_sha256().to_owned(),
        dependency_results: dependencies.clone(),
        output_files,
        result_artifact: result,
        result_artifact_sha256: result_sha256.clone(),
        mediator_witness_sha256: String::new(),
    };
    reuse.mediator_witness_sha256 = reuse_witness(&reuse)?;
    Ok(IntentResult::Executed(ExecutedArtifact {
        result_sha256,
        reuse_bytes: canonical(&reuse)?,
    }))
}

fn expected_rust_source_syntax_stdout(frame: &[u8]) -> Result<Vec<u8>, RoutineError> {
    let RustSourceSyntaxOutcome::Passed(observation) = evaluate_rust_source_syntax_frame(frame)
    else {
        return Err(mediator_error("mediator-rust-source-behavior-refused"));
    };
    let mut bytes = rust_source_syntax_observation_json(&observation);
    bytes.push(b'\n');
    Ok(bytes)
}
