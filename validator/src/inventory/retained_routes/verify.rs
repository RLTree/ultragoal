#[cfg(test)]
use super::model::AUTHORITY_CLASSIFICATION;
use super::model::{
    ACTIVE_READER_WRITER_STATE, COMPATIBILITY_BEHAVIOR, COMPATIBILITY_BOUNDARY, CatalogEvidence,
    EQUIVALENCE_PROOF, EntryEvidence, INTENDED_DISPOSITION, OBSERVED_AUTHORITY_STATE,
    PHYSICAL_CLEANUP_STATE, REPLACEMENT_STATE, RegistryRouteEvidence, RouteSpec,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VerificationError {
    UnknownRoute,
    RouteSetMismatch,
    RegistryMismatch,
    UnregisteredSource,
    SourceSetMismatch,
    SourceMismatch,
    SourceBytesMismatch,
    DigestSetMismatch,
    TargetMissing,
    TargetSetMismatch,
    TargetMismatch,
    ActiveCanonicalImplementation,
    Conflict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedPendingAuthority {
    pub spec: &'static RouteSpec,
}

impl VerifiedPendingAuthority {
    #[cfg(test)]
    pub(crate) const fn classification(self) -> &'static str {
        AUTHORITY_CLASSIFICATION
    }

    /// This does not demote the source. It authorizes a distinct pending-
    /// migration finding only while the exact source, definition-only target,
    /// registry row, and current source bytes remain jointly verified.
    pub(crate) const fn reclassifies_parallel_authority(self) -> bool {
        true
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct VerifiedCatalog {
    pub routes: Vec<VerifiedPendingAuthority>,
}

#[cfg(test)]
pub(crate) fn registry_route_is_compiled(route: &RegistryRouteEvidence<'_>) -> bool {
    super::specs::by_route_id(route.route_id).is_some_and(|spec| registry_matches(spec, route))
}

pub(crate) fn verify_catalog(
    evidence: CatalogEvidence<'_>,
) -> Result<VerifiedCatalog, VerificationError> {
    reject_raw_conflicts(&evidence)?;
    reject_unknown_rows(&evidence)?;
    if evidence.raw_sources.len() != super::specs::ROUTE_COUNT {
        return Err(VerificationError::SourceSetMismatch);
    }
    if evidence.raw_targets.len() != super::specs::TARGET_COUNT {
        return Err(VerificationError::TargetSetMismatch);
    }
    if evidence.raw_registry_routes.len() != super::specs::ROUTE_COUNT {
        return Err(VerificationError::RouteSetMismatch);
    }
    if evidence.current_sources.len() != super::specs::ROUTE_COUNT {
        return Err(VerificationError::DigestSetMismatch);
    }

    let mut verified = Vec::with_capacity(super::specs::ROUTE_COUNT);
    for spec in super::specs::routes() {
        let source = exactly_one_source(evidence.raw_sources, spec)?;
        let registry = exactly_one_registry(evidence.raw_registry_routes, spec)?;
        let target = exactly_one_target(evidence.raw_targets, spec)?;
        let digest = evidence
            .current_sources
            .iter()
            .find(|digest| digest.path == spec.path)
            .ok_or(VerificationError::DigestSetMismatch)?;
        if !source_matches(spec, source) {
            return Err(VerificationError::SourceMismatch);
        }
        if digest.sha256 != spec.source_sha256 {
            return Err(VerificationError::SourceBytesMismatch);
        }
        if target.active_status == "active" {
            return Err(VerificationError::ActiveCanonicalImplementation);
        }
        if !target_matches(spec, target) {
            return Err(VerificationError::TargetMismatch);
        }
        if !registry_matches(spec, registry) {
            return Err(VerificationError::RegistryMismatch);
        }
        verified.push(VerifiedPendingAuthority { spec });
    }
    Ok(VerifiedCatalog { routes: verified })
}

fn reject_raw_conflicts(evidence: &CatalogEvidence<'_>) -> Result<(), VerificationError> {
    let sources = evidence.raw_sources;
    let targets = evidence.raw_targets;
    if duplicate_entries(sources) || duplicate_entries(targets) {
        return Err(VerificationError::Conflict);
    }
    if sources.iter().any(|source| {
        targets
            .iter()
            .any(|target| source.stable_id == target.stable_id || source.path == target.path)
    }) {
        return Err(VerificationError::Conflict);
    }
    let mut route_ids = BTreeSet::new();
    let mut stable_ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for route in evidence.raw_registry_routes {
        let Some(stable_id) = route.matcher.stable_id else {
            continue;
        };
        let Some(path) = route.matcher.relative_path else {
            continue;
        };
        if !route_ids.insert(route.route_id) || !stable_ids.insert(stable_id) || !paths.insert(path)
        {
            return Err(VerificationError::Conflict);
        }
    }
    let mut digest_paths = BTreeSet::new();
    if evidence
        .current_sources
        .iter()
        .any(|digest| !digest_paths.insert(digest.path))
    {
        return Err(VerificationError::Conflict);
    }
    Ok(())
}

fn duplicate_entries(entries: &[EntryEvidence<'_>]) -> bool {
    let mut stable_ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    entries
        .iter()
        .any(|entry| !stable_ids.insert(entry.stable_id) || !paths.insert(entry.path))
}

fn reject_unknown_rows(evidence: &CatalogEvidence<'_>) -> Result<(), VerificationError> {
    if evidence.raw_sources.iter().any(|source| {
        super::specs::by_stable_id(source.stable_id)
            .is_none_or(|spec| spec.kind != source.kind || spec.path != source.path)
    }) {
        return Err(VerificationError::UnregisteredSource);
    }
    if evidence
        .raw_registry_routes
        .iter()
        .any(|route| super::specs::by_route_id(route.route_id).is_none())
    {
        return Err(VerificationError::UnknownRoute);
    }
    Ok(())
}

fn exactly_one_source<'a>(
    rows: &'a [EntryEvidence<'a>],
    spec: &RouteSpec,
) -> Result<&'a EntryEvidence<'a>, VerificationError> {
    rows.iter()
        .find(|source| source.stable_id == spec.stable_id)
        .ok_or(VerificationError::SourceSetMismatch)
}

fn exactly_one_target<'a>(
    rows: &'a [EntryEvidence<'a>],
    spec: &RouteSpec,
) -> Result<&'a EntryEvidence<'a>, VerificationError> {
    rows.iter()
        .find(|target| target.stable_id == spec.target.stable_id)
        .ok_or(VerificationError::TargetMissing)
}

fn exactly_one_registry<'a>(
    rows: &'a [RegistryRouteEvidence<'a>],
    spec: &RouteSpec,
) -> Result<&'a RegistryRouteEvidence<'a>, VerificationError> {
    rows.iter()
        .find(|route| route.route_id == spec.route_id)
        .ok_or(VerificationError::RouteSetMismatch)
}

fn registry_matches(spec: &RouteSpec, route: &RegistryRouteEvidence<'_>) -> bool {
    route.matcher.stable_id == Some(spec.stable_id)
        && route.matcher.kind == Some(spec.kind)
        && route.matcher.relative_path == Some(spec.path)
        && route.canonical_target == spec.target.stable_id
        && route.intended_disposition == INTENDED_DISPOSITION
        && transition_matches(spec, route)
}

fn transition_matches(spec: &RouteSpec, route: &RegistryRouteEvidence<'_>) -> bool {
    let transition = route.transition;
    transition.compatibility_behavior == COMPATIBILITY_BEHAVIOR
        && transition.compatibility_boundary == COMPATIBILITY_BOUNDARY
        && transition.replacement_state == REPLACEMENT_STATE
        && transition.active_reader_writer_state == ACTIVE_READER_WRITER_STATE
        && transition.observed_authority_state == OBSERVED_AUTHORITY_STATE
        && transition.equivalence_proof == EQUIVALENCE_PROOF
        && transition.physical_cleanup_state == PHYSICAL_CLEANUP_STATE
        && transition.proof_refs == spec.proof_refs
}

fn source_matches(spec: &RouteSpec, source: &EntryEvidence<'_>) -> bool {
    source.stable_id == spec.stable_id
        && source.kind == spec.kind
        && source.path == spec.path
        && source.sha256 == spec.source_sha256
        && source.authority_state == "legacy"
        && source.active_status == "active"
}

fn target_matches(spec: &RouteSpec, target: &EntryEvidence<'_>) -> bool {
    target.stable_id == spec.target.stable_id
        && target.kind == spec.target.kind
        && target.path == spec.target.path
        && target.sha256 == spec.target.sha256
        && target.authority_state == "canonical"
        && target.active_status == "definition"
}
