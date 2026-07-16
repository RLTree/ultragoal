use super::*;

pub(crate) fn result_matches_reuse(wire: &ReuseArtifactWire) -> bool {
    let result = &wire.result_artifact;
    result.schema_version == "RoutineMediatedResultArtifact-v2"
        && result.protocol_id == wire.protocol_id
        && result.intent_id == wire.intent_id
        && result.node_id == wire.node_id
        && result.behavior_id == wire.behavior_id
        && result.plan_order == wire.plan_order
        && result.context_id == wire.context_id
        && result.candidate_id == wire.candidate_id
        && result.plan_id == wire.plan_id
        && result.snapshot_id == wire.snapshot_id
        && result.input_id == wire.input_id
        && result.tool_identity_sha256 == wire.tool_identity_sha256
        && result.program_sha256 == wire.program_sha256
        && result.environment_sha256 == wire.environment_sha256
        && result.read_authority_sha256 == wire.read_authority_sha256
        && result.dependency_results == wire.dependency_results
        && result.output_files == wire.output_files
        && valid(&result.behavior_sha256)
}

pub(crate) fn reuse_witness(wire: &ReuseArtifactWire) -> Result<String, RoutineError> {
    #[derive(Serialize)]
    struct Witness<'a> {
        domain: &'static str,
        schema_version: &'a str,
        state: &'a str,
        protocol_id: &'a str,
        intent_id: &'a str,
        node_id: &'a str,
        behavior_id: &'a str,
        plan_order: usize,
        context_id: &'a str,
        candidate_id: &'a str,
        plan_id: &'a str,
        snapshot_id: &'a str,
        input_id: &'a str,
        tool_identity_sha256: &'a str,
        program_sha256: &'a str,
        environment_sha256: &'a str,
        read_authority_sha256: &'a str,
        dependency_results: &'a BTreeMap<String, String>,
        output_files: &'a BTreeMap<String, outcome::OutputFileRecord>,
        result_artifact_sha256: &'a str,
    }
    digest_of(&Witness {
        domain: "routine-mediated-reuse-witness-v2",
        schema_version: &wire.schema_version,
        state: &wire.state,
        protocol_id: &wire.protocol_id,
        intent_id: &wire.intent_id,
        node_id: &wire.node_id,
        behavior_id: &wire.behavior_id,
        plan_order: wire.plan_order,
        context_id: &wire.context_id,
        candidate_id: &wire.candidate_id,
        plan_id: &wire.plan_id,
        snapshot_id: &wire.snapshot_id,
        input_id: &wire.input_id,
        tool_identity_sha256: &wire.tool_identity_sha256,
        program_sha256: &wire.program_sha256,
        environment_sha256: &wire.environment_sha256,
        read_authority_sha256: &wire.read_authority_sha256,
        dependency_results: &wire.dependency_results,
        output_files: &wire.output_files,
        result_artifact_sha256: &wire.result_artifact_sha256,
    })
}

pub(crate) fn reconcile_internal(
    context: &LiveContext,
    plan: &RoutinePlan,
    authority: &RoutineMediationAuthority,
    nodes: &[RoutineNodeMediation],
) -> Result<(), RoutineError> {
    context
        .revalidate()
        .map_err(|_| concurrent("mediator-result-context-stale"))?;
    let current = RoutineBinding::from_live(context)?;
    if &current != plan.binding()
        || authority.binding != current
        || authority.plan_id != plan.plan_id()
        || authority.snapshot_id != plan.snapshot_id()
        || nodes.len() != authority.expected.len()
    {
        return Err(mediator_error("mediator-result-binding-invalid"));
    }
    for ((expected, node), check) in authority.expected.iter().zip(nodes).zip(plan.checks()) {
        if expected.intent_id != node.intent_id
            || expected.plan_order != node.plan_order
            || expected.node_id != node.node_id
            || check.node_id() != node.node_id
            || matches!(
                node.disposition,
                RoutineNodeDisposition::Executed | RoutineNodeDisposition::Reused
            ) != node.result_artifact_sha256.as_deref().is_some_and(valid)
            || matches!(
                node.disposition,
                RoutineNodeDisposition::Failed
                    | RoutineNodeDisposition::DependencyFailed
                    | RoutineNodeDisposition::Cancelled
            ) != node.failure_code.is_some()
        {
            return Err(mediator_error(
                "mediator-result-order-or-cardinality-invalid",
            ));
        }
    }
    authority.require_complete()?;
    Ok(())
}

pub(crate) fn success_node(
    token: &RoutineMediatedIntent,
    disposition: RoutineNodeDisposition,
    result_artifact_sha256: String,
) -> RoutineNodeMediation {
    RoutineNodeMediation {
        intent_id: token.intent().intent_id().to_owned(),
        node_id: token.intent().node_id().to_owned(),
        plan_order: token.intent().plan_order(),
        disposition,
        result_artifact_sha256: Some(result_artifact_sha256),
        failure_code: None,
    }
}
