use super::artifact_model::CapturedArtifact;
use super::artifact_safety;
#[cfg(test)]
use super::environment;
use super::environment::InvocationSensitivity;
use super::filesystem;
use super::filesystem::RootAnchor;
#[cfg(test)]
use super::filesystem::validate_context_roots;
#[cfg(test)]
use super::identity_codec::context_candidate_id;
use super::identity_codec::{bytes_hex, valid_sha256};
use super::inputs::ArtifactExpectation;
#[cfg(test)]
use super::inputs::PublicArtifact;
use super::path_policy::{os_bytes, validate_public_path, validate_relative};
#[cfg(test)]
use super::spec::CommandSpec;
use super::spec::MAX_PATH_BYTES;
#[cfg(test)]
use crate::context::LiveContext;
use std::collections::BTreeSet;

const MAX_ARTIFACT_BYTES: usize = 4 * 1024 * 1024;
const MAX_AGGREGATE_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;

pub(super) fn validate_expectations(items: &[ArtifactExpectation]) -> Result<(), String> {
    let mut paths = BTreeSet::new();
    for item in items {
        let ArtifactExpectation::Public(item) = item else {
            if let ArtifactExpectation::Secret(secret) = item {
                let _ = (&secret.path, &secret.source);
            }
            return Err("secret artifact capture is unsupported and was not read".to_owned());
        };
        if os_bytes(item.path.as_os_str()).len() > MAX_PATH_BYTES {
            return Err("artifact path exceeds the supported path byte bound".to_owned());
        }
        validate_relative(&item.path, "artifact path")?;
        validate_public_path(&item.path, "artifact path")?;
        let encoded = bytes_hex(os_bytes(item.path.as_os_str()));
        if !paths.insert(encoded) {
            return Err("duplicate artifact path".to_owned());
        }
        if item
            .expected_sha256
            .as_deref()
            .is_some_and(|digest| !valid_sha256(digest))
        {
            return Err("artifact expected digest is not canonical sha256".to_owned());
        }
    }
    Ok(())
}

pub(super) fn capture(
    root: &RootAnchor,
    items: &[ArtifactExpectation],
    context_id: &str,
    candidate_id: &str,
    sensitivity: InvocationSensitivity,
) -> Result<Vec<CapturedArtifact>, String> {
    let mut observed = 0_usize;
    let mut artifacts = Vec::with_capacity(items.len());
    for expectation in items {
        let ArtifactExpectation::Public(item) = expectation else {
            return Err("secret artifact capture is unsupported and was not read".to_owned());
        };
        if sensitivity.is_secret_bearing() {
            let finalized = artifact_safety::withheld_secret_bearing_invocation();
            artifacts.push(CapturedArtifact {
                schema_version: "CapturedArtifact-v1",
                context_id: context_id.to_owned(),
                candidate_id: candidate_id.to_owned(),
                relative_path_hex: None,
                content_disposition: finalized.disposition,
                content_sha256: finalized.sha256,
                byte_length: finalized.bytes.len() as u64,
                store_identity: "immutable-memory-arc-v1",
                resolver_identity: "captured-artifact-sealed-v1",
                relative_path: None,
                bytes: finalized.bytes,
            });
            continue;
        }
        let remaining = MAX_AGGREGATE_ARTIFACT_BYTES.saturating_sub(observed);
        let read_limit = remaining.min(MAX_ARTIFACT_BYTES);
        let bound_error = if remaining < MAX_ARTIFACT_BYTES {
            "aggregate artifact bytes exceed the supported bound"
        } else {
            "artifact exceeds the supported per-file byte bound"
        };
        let pinned = root
            .open_regular(&item.path, false, read_limit)
            .map_err(|error| {
                if error.contains("configured read bound before read") {
                    bound_error.to_owned()
                } else {
                    error
                }
            })?;
        let expected_length = usize::try_from(pinned.byte_length()?)
            .map_err(|_| "artifact length exceeds the supported platform bound".to_owned())?;
        observed = observed
            .checked_add(expected_length)
            .filter(|total| *total <= MAX_AGGREGATE_ARTIFACT_BYTES)
            .ok_or_else(|| "aggregate artifact bytes exceed the supported bound".to_owned())?;
        filesystem::test_artifact_pause();
        let bytes = pinned.bytes();
        let sha256 = pinned.sha256().to_owned();
        pinned.validate(root, false)?;
        if bytes.len() != expected_length {
            return Err("artifact length changed during capture".to_owned());
        }
        if item
            .expected_sha256
            .as_ref()
            .is_some_and(|expected| expected != &sha256)
        {
            return Err("artifact content did not match its required digest".to_owned());
        }
        let finalized = artifact_safety::public(bytes, sha256);
        artifacts.push(CapturedArtifact {
            schema_version: "CapturedArtifact-v1",
            context_id: context_id.to_owned(),
            candidate_id: candidate_id.to_owned(),
            relative_path_hex: Some(bytes_hex(os_bytes(item.path.as_os_str()))),
            content_disposition: finalized.disposition,
            content_sha256: finalized.sha256,
            byte_length: finalized.bytes.len() as u64,
            store_identity: "immutable-memory-arc-v1",
            resolver_identity: "captured-artifact-sealed-v1",
            relative_path: Some(item.path.clone()),
            bytes: finalized.bytes,
        });
    }
    Ok(artifacts)
}

#[cfg(test)]
pub fn capture_public_for_test(
    context: &LiveContext,
    items: Vec<PublicArtifact>,
) -> Result<Vec<CapturedArtifact>, String> {
    validate_context_roots(context)?;
    context
        .revalidate()
        .map_err(|_| "test artifact context failed revalidation".to_owned())?;
    let expectations = items
        .into_iter()
        .map(ArtifactExpectation::Public)
        .collect::<Vec<_>>();
    validate_expectations(&expectations)?;
    let root = RootAnchor::new(context)?;
    let candidate_id = context_candidate_id(context)?;
    let artifacts = capture(
        &root,
        &expectations,
        context.context_id(),
        &candidate_id,
        InvocationSensitivity::Public,
    )?;
    root.validate()?;
    context
        .revalidate()
        .map_err(|_| "test artifact context drifted".to_owned())?;
    Ok(artifacts)
}

#[cfg(test)]
pub fn capture_spec_artifacts_for_test(
    context: &LiveContext,
    spec: &CommandSpec,
) -> Result<Vec<CapturedArtifact>, String> {
    spec.validate()?;
    validate_context_roots(context)?;
    context
        .revalidate()
        .map_err(|_| "test artifact context failed revalidation".to_owned())?;
    let prepared = environment::prepare(spec, context)?;
    let root = RootAnchor::new(context)?;
    let candidate_id = context_candidate_id(context)?;
    let artifacts = capture(
        &root,
        &spec.artifacts,
        context.context_id(),
        &candidate_id,
        prepared.sensitivity,
    )?;
    root.validate()?;
    context
        .revalidate()
        .map_err(|_| "test artifact context drifted".to_owned())?;
    Ok(artifacts)
}
