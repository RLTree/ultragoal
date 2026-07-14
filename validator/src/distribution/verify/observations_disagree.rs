fn observations_disagree(left: Layer, right: Layer, rows: &BTreeMap<Layer, Observation>) -> bool {
    let (Some(left), Some(right)) = (rows.get(&left), rows.get(&right)) else {
        return false;
    };
    left.envelope.context_id != right.envelope.context_id
        || left.envelope.candidate_id != right.envelope.candidate_id
        || left.envelope.plugin_id != right.envelope.plugin_id
        || left.envelope.version != right.envelope.version
        || (left.envelope.artifact_sha256.is_some()
            && right.envelope.artifact_sha256.is_some()
            && left.envelope.artifact_sha256 != right.envelope.artifact_sha256)
}

fn report_digest(
    context: &str,
    candidate: &str,
    host: HostVerdict,
    layers: &[LayerReport],
    joins: &[JoinReport],
) -> String {
    #[derive(Serialize)]
    struct DigestInput<'a> {
        schema: &'static str,
        context_id: &'a str,
        candidate_id: &'a str,
        host_verdict: HostVerdict,
        layers: &'a [LayerReport],
        joins: &'a [JoinReport],
    }
    let bytes = serde_json::to_vec(&DigestInput {
        schema: "harness-ultragoal.distribution-report.v1",
        context_id: context,
        candidate_id: candidate,
        host_verdict: host,
        layers,
        joins,
    })
    .expect("serializable distribution report");
    sha256(&bytes)
}

fn current_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "other"
    }
}

pub fn verify_identity_ladder(bytes: &[u8]) -> Result<Vec<SurfaceIdentity>, DistributionError> {
    let rows = crate::distribution::spec::parse_identity_ladder(bytes)?;
    verify_surface_chain(&rows)?;
    Ok(rows)
}
