use super::*;

#[path = "reuse_input_index/authentication_authority.rs"]
mod authentication_authority;

pub(crate) fn index_reuse_inputs(
    input: RoutineReuseInput,
    request: &RoutineEffectRequest,
) -> Result<BTreeMap<String, Vec<u8>>, RoutineError> {
    let known = request
        .intents
        .iter()
        .map(|intent| intent.intent_id())
        .collect::<BTreeSet<_>>();
    let mut indexed = BTreeMap::new();
    for bytes in input.into_artifacts() {
        let Ok(wire) = serde_json::from_slice::<ReuseArtifactWire>(&bytes) else {
            continue;
        };
        if wire.protocol_id == request.protocol_id
            && known.contains(wire.intent_id.as_str())
            && wire.state != "complete"
        {
            return Err(mediator_error("mediator-ambiguous-started-artifact"));
        }
        if wire.state == "complete" && known.contains(wire.intent_id.as_str()) {
            if indexed.insert(wire.intent_id, bytes).is_some() {
                return Err(mediator_error("mediator-reuse-artifact-duplicated"));
            }
        }
    }
    Ok(indexed)
}

pub(crate) struct ReuseArtifactValidation<'a> {
    pub(crate) context: &'a LiveContext,
    pub(crate) plan: &'a RoutinePlan,
    pub(crate) token: &'a RoutineMediatedIntent,
    pub(crate) snapshot_id: &'a str,
    pub(crate) dependencies: &'a BTreeMap<String, String>,
    pub(crate) outputs: &'a OutputConfinement,
    pub(crate) attempt: &'a AttemptReservation,
}

pub(crate) fn verify_reuse_artifact(
    bytes: &[u8],
    validation: ReuseArtifactValidation<'_>,
) -> Result<Option<VerifiedReuseArtifact>, RoutineError> {
    let ReuseArtifactValidation {
        context,
        plan,
        token,
        snapshot_id,
        dependencies,
        outputs,
        attempt,
    } = validation;
    let Ok(wire) = serde_json::from_slice::<ReuseArtifactWire>(bytes) else {
        return Ok(None);
    };
    if canonical(&wire)? != bytes
        || wire.schema_version != "RoutineMediatedReuseArtifact-v2"
        || wire.state != "complete"
    {
        return Ok(None);
    }
    let artifact_sha256 = sha256(bytes);
    if !attempt.authenticates_artifact(&artifact_sha256, &wire.mediator_witness_sha256)?
        || wire.mediator_witness_sha256 != reuse_witness(&wire)?
        || wire.protocol_id != token.protocol_id()
        || wire.intent_id != token.intent().intent_id()
        || wire.node_id != token.intent().node_id()
        || wire.behavior_id != token.intent().behavior_id()
        || wire.plan_order != token.intent().plan_order()
        || wire.context_id != context.context_id()
        || wire.candidate_id != plan.binding().candidate_id()
        || wire.plan_id != plan.plan_id()
        || wire.snapshot_id != snapshot_id
        || wire.input_id != token.intent().input_id()
        || wire.tool_identity_sha256 != token.intent().tool_identity_sha256()
        || wire.program_sha256 != token.intent().program_sha256()
        || wire.environment_sha256 != token.intent().environment_sha256()
        || wire.read_authority_sha256 != token.intent().read_authority_sha256()
        || wire.dependency_results != *dependencies
        || wire.result_artifact_sha256 != sha256(&canonical(&wire.result_artifact)?)
        || !result_matches_reuse(&wire)
        || outputs.capture_owned_delta()? != wire.output_files
    {
        return Ok(None);
    }
    Ok(Some(VerifiedReuseArtifact {
        wire,
        canonical_bytes: bytes.to_vec(),
    }))
}

#[cfg(test)]
#[path = "reuse_input_index/durable_authentication_fixture.rs"]
mod durable_authentication_fixture;
#[cfg(test)]
#[path = "reuse_input_index/durable_authority_tests.rs"]
mod durable_authority_tests;

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
