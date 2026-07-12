use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::model::{
    DistributionReport, HostVerdict, JoinReport, JoinVerdict, Layer, LayerReport, LayerVerdict,
    SurfaceIdentity, verify_surface_chain,
};
use crate::distribution::reader::{ReadSession, sha256};
use crate::distribution::spec::{
    Envelope, ObservationState, PAYLOAD_LIMIT, Request, parse_envelope, parse_request,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug)]
struct Observation {
    envelope: Envelope,
    payload_sha256: Option<String>,
}

pub fn verify(root: &Path, request_bytes: &[u8]) -> Result<DistributionReport, DistributionError> {
    let request = parse_request(request_bytes)?;
    let current = current_platform();
    let host_verdict = if request.platform == current {
        HostVerdict::Supported
    } else {
        if request
            .layers
            .values()
            .any(|row| row.state != ObservationState::Unavailable)
        {
            return Err(error(DistributionErrorId::CapabilityMismatch));
        }
        HostVerdict::PlatformUnavailable
    };

    let observations = read_observations(root, &request)?;
    let anchor = observations.get(&Layer::SourcePluginMetadata);
    let plugin_id = anchor.map(|row| row.envelope.plugin_id.as_str());
    let version = anchor.map(|row| row.envelope.version.as_str());
    let artifact = observations
        .get(&Layer::SourcePackageInput)
        .and_then(|row| row.envelope.artifact_sha256.as_deref());

    let mut layers = Vec::with_capacity(Layer::ALL.len());
    for layer in Layer::ALL {
        let spec = request.layers.get(&layer).expect("validated layer set");
        let verdict = match spec.state {
            ObservationState::DefinitionOnly => LayerVerdict::DefinitionOnly,
            ObservationState::Missing => LayerVerdict::Missing,
            ObservationState::Unavailable => LayerVerdict::Unavailable,
            ObservationState::Observed => observation_verdict(
                layer,
                observations.get(&layer).expect("observed layer loaded"),
                &request,
                plugin_id,
                version,
                artifact,
            ),
        };
        layers.push(LayerReport { layer, verdict });
    }
    for index in 1..layers.len() {
        if layers[index].verdict == LayerVerdict::Verified
            && layers[index - 1].verdict != LayerVerdict::Verified
        {
            layers[index].verdict = LayerVerdict::ObservedUnjoined;
        }
    }
    let joins = layers
        .windows(2)
        .map(|pair| JoinReport {
            from: pair[0].layer,
            to: pair[1].layer,
            verdict: if pair[0].verdict == LayerVerdict::Verified
                && pair[1].verdict == LayerVerdict::Verified
            {
                JoinVerdict::Match
            } else if observations_disagree(pair[0].layer, pair[1].layer, &observations) {
                JoinVerdict::Mismatch
            } else {
                JoinVerdict::NotEvaluated
            },
        })
        .collect::<Vec<_>>();
    let ladder_sha256 = report_digest(
        &request.context_id,
        &request.candidate_id,
        host_verdict,
        &layers,
        &joins,
    );
    Ok(DistributionReport {
        context_id: request.context_id,
        candidate_id: request.candidate_id,
        host_verdict,
        layers,
        joins,
        ladder_sha256,
    })
}

fn read_observations(
    root: &Path,
    request: &Request,
) -> Result<BTreeMap<Layer, Observation>, DistributionError> {
    if request
        .layers
        .values()
        .all(|row| row.state != ObservationState::Observed)
    {
        return Ok(BTreeMap::new());
    }
    let mut session = ReadSession::open(root)?;
    let mut observations = BTreeMap::new();
    for layer in Layer::ALL {
        let spec = request.layers.get(&layer).expect("validated layer set");
        if spec.state != ObservationState::Observed {
            continue;
        }
        let envelope = parse_envelope(
            session
                .read(
                    spec.identity_path
                        .as_deref()
                        .expect("validated identity path"),
                    super::spec::IDENTITY_LIMIT,
                )?
                .as_slice(),
        )?;
        let payload_sha256 = spec
            .payload_path
            .as_deref()
            .map(|path| {
                session
                    .read(path, PAYLOAD_LIMIT)
                    .map(|bytes| sha256(&bytes))
            })
            .transpose()?;
        observations.insert(
            layer,
            Observation {
                envelope,
                payload_sha256,
            },
        );
    }
    session.finish()?;
    Ok(observations)
}

fn observation_verdict(
    layer: Layer,
    row: &Observation,
    request: &Request,
    plugin_id: Option<&str>,
    version: Option<&str>,
    artifact: Option<&str>,
) -> LayerVerdict {
    let envelope = &row.envelope;
    if envelope.layer != layer {
        return LayerVerdict::Contradicted;
    }
    if envelope.context_id != request.context_id || envelope.candidate_id != request.candidate_id {
        return LayerVerdict::MixedCandidate;
    }
    if envelope.plugin_id != "harness-ultragoal"
        || plugin_id.is_some_and(|expected| envelope.plugin_id != expected)
        || version.is_some_and(|expected| envelope.version != expected)
    {
        return LayerVerdict::StaleIdentity;
    }
    if let Some(payload) = row.payload_sha256.as_deref() {
        if envelope.artifact_sha256.as_deref() != Some(payload) {
            return LayerVerdict::PayloadMismatch;
        }
    }
    if layer != Layer::SourcePluginMetadata
        && artifact.is_some()
        && envelope.artifact_sha256.as_deref() != artifact
    {
        return LayerVerdict::StaleIdentity;
    }
    let expected_exposure = if layer.requires_exposure() {
        "affirmed"
    } else {
        "not-applicable"
    };
    if envelope.exposure != expected_exposure {
        return LayerVerdict::Contradicted;
    }
    LayerVerdict::Verified
}

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
