use super::super::*;
use super::routine_mediation_result::OutputFileRecord;

pub(in crate::routine_work::runtime_adapter::mediator) struct ExecutedIntentProjection<'a> {
    pub(in crate::routine_work::runtime_adapter::mediator) snapshot_id: &'a str,
    pub(in crate::routine_work::runtime_adapter::mediator) dependencies:
        &'a BTreeMap<String, String>,
    pub(in crate::routine_work::runtime_adapter::mediator) framed_input_sha256: &'a str,
    pub(in crate::routine_work::runtime_adapter::mediator) observation: &'a ProcessObservation,
    pub(in crate::routine_work::runtime_adapter::mediator) output_files:
        BTreeMap<String, OutputFileRecord>,
}

pub(in crate::routine_work::runtime_adapter::mediator) fn project_executed_intent(
    context: &LiveContext,
    plan: &RoutinePlan,
    token: &RoutineMediatedIntent,
    projection: ExecutedIntentProjection<'_>,
) -> Result<ExecutedArtifact, RoutineError> {
    let behavior_sha256 = framed(&[
        RESULT_DOMAIN,
        token.intent().behavior_id().as_bytes(),
        projection.framed_input_sha256.as_bytes(),
        &projection.observation.stdout,
        projection.observation.stderr_sha256.as_bytes(),
        digest_of(&projection.output_files)?.as_bytes(),
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
        snapshot_id: projection.snapshot_id.to_owned(),
        input_id: token.intent().input_id().to_owned(),
        tool_identity_sha256: token.intent().tool_identity_sha256().to_owned(),
        program_sha256: token.intent().program_sha256().to_owned(),
        environment_sha256: token.intent().environment_sha256().to_owned(),
        read_authority_sha256: token.intent().read_authority_sha256().to_owned(),
        dependency_results: projection.dependencies.clone(),
        behavior_sha256,
        output_files: projection.output_files.clone(),
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
        snapshot_id: projection.snapshot_id.to_owned(),
        input_id: token.intent().input_id().to_owned(),
        tool_identity_sha256: token.intent().tool_identity_sha256().to_owned(),
        program_sha256: token.intent().program_sha256().to_owned(),
        environment_sha256: token.intent().environment_sha256().to_owned(),
        read_authority_sha256: token.intent().read_authority_sha256().to_owned(),
        dependency_results: projection.dependencies.clone(),
        output_files: projection.output_files,
        result_artifact: result,
        result_artifact_sha256: result_sha256.clone(),
        mediator_witness_sha256: String::new(),
    };
    reuse.mediator_witness_sha256 = reuse_witness(&reuse)?;
    Ok(ExecutedArtifact {
        node: success_node(
            token,
            RoutineNodeDisposition::Executed,
            result_sha256.clone(),
        ),
        result_sha256,
        reuse_bytes: canonical(&reuse)?,
    })
}
