use super::{RouteRule, RoutingData};
use crate::context::ReadSession;
use crate::inventory::digest::sha256_hex;
use crate::inventory::fs::read_bounded;
use crate::inventory::retained_routes::{
    CatalogEvidence, DigestEvidence, EntryEvidence, MatcherEvidence, RegistryRouteEvidence,
    TransitionEvidence, by_stable_id, is_source_kind, is_target_id, verify_catalog,
};
use crate::inventory::routing_state::{
    CompatibilityBehavior, CompatibilityBoundary, EquivalenceProof, ObservedAuthorityState,
    PhysicalCleanupState, ReaderWriterState, ReplacementState,
};
use crate::inventory::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use std::collections::BTreeSet;
use std::path::Path;

const MAX_PENDING_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

fn compatibility_behavior(value: CompatibilityBehavior) -> &'static str {
    match value {
        CompatibilityBehavior::Unverified => "unverified",
        CompatibilityBehavior::ExactRouteOnly => "exact-route-only",
        CompatibilityBehavior::Verified => "verified",
        CompatibilityBehavior::NotApplicable => "not-applicable",
    }
}

fn compatibility_boundary(value: CompatibilityBoundary) -> &'static str {
    match value {
        CompatibilityBoundary::BlockedByOd008 => "blocked-by-OD-008",
        CompatibilityBoundary::ExplicitOnly => "explicit-only",
        CompatibilityBoundary::Adopted => "adopted",
        CompatibilityBoundary::NotApplicable => "not-applicable",
    }
}

fn replacement_state(value: ReplacementState) -> &'static str {
    match value {
        ReplacementState::Unverified => "unverified",
        ReplacementState::CandidateRequired => "candidate-required",
        ReplacementState::Verified => "verified",
    }
}

fn reader_writer_state(value: ReaderWriterState) -> &'static str {
    match value {
        ReaderWriterState::Active => "active",
        ReaderWriterState::Unknown => "unknown",
        ReaderWriterState::NoneVerified => "none-verified",
    }
}

fn observed_authority_state(value: ObservedAuthorityState) -> &'static str {
    match value {
        ObservedAuthorityState::Active => "active",
        ObservedAuthorityState::CompatibilityRouteRetained => "compatibility-route-retained",
        ObservedAuthorityState::ContextOnly => "context-only",
        ObservedAuthorityState::Archived => "archived",
    }
}

fn equivalence_proof(value: EquivalenceProof) -> &'static str {
    match value {
        EquivalenceProof::Missing => "missing",
        EquivalenceProof::Verified => "verified",
        EquivalenceProof::NotApplicable => "not-applicable",
    }
}

fn physical_cleanup_state(value: PhysicalCleanupState) -> &'static str {
    match value {
        PhysicalCleanupState::BlockedByOd009 => "blocked-by-OD-009",
        PhysicalCleanupState::Preserve => "preserve",
        PhysicalCleanupState::Authorized => "authorized",
    }
}

fn pending_route_candidate(route: &RouteRule) -> bool {
    route
        .matcher
        .stable_id
        .as_deref()
        .is_some_and(|stable_id| by_stable_id(stable_id).is_some())
        || route.matcher.kind.as_deref().is_some_and(is_source_kind)
}

fn entry_evidence(entry: &InventoryEntry) -> EntryEvidence<'_> {
    EntryEvidence {
        stable_id: &entry.stable_id,
        kind: &entry.kind,
        path: &entry.relative_path,
        sha256: &entry.digest_sha256,
        authority_state: match entry.authority_state {
            AuthorityState::Legacy => "legacy",
            AuthorityState::Canonical => "canonical",
            AuthorityState::Projection => "projection",
            AuthorityState::Context => "context",
        },
        active_status: match entry.active_status {
            ActiveStatus::Active => "active",
            ActiveStatus::Candidate => "candidate",
            ActiveStatus::Definition => "definition",
            ActiveStatus::Required => "required",
            ActiveStatus::Missing => "missing",
            ActiveStatus::Retired => "retired",
            ActiveStatus::ContextOnly => "context-only",
        },
    }
}

fn verification_failed(
    findings: &mut Vec<InventoryFinding>,
    error: impl std::fmt::Debug,
) -> BTreeSet<String> {
    findings.push(InventoryFinding::error(
        "pending_authority_verification_failed",
        None,
        None,
        format!(
            "exact sole-current authority catalog failed closed ({error:?}); affected legacy sources remain parallel-authority findings"
        ),
    ));
    BTreeSet::new()
}

impl RoutingData {
    pub(crate) fn classify_pending_authority(
        &self,
        reads: &ReadSession,
        root: &Path,
        groups: &[Vec<InventoryEntry>],
        findings: &mut Vec<InventoryFinding>,
    ) -> Result<BTreeSet<String>, InventoryError> {
        let source_rows = groups
            .iter()
            .flatten()
            .filter(|entry| is_source_kind(&entry.kind))
            .collect::<Vec<_>>();
        let target_rows = groups
            .iter()
            .flatten()
            .filter(|entry| is_target_id(&entry.stable_id))
            .collect::<Vec<_>>();
        let route_rows = self
            .registry
            .routes
            .iter()
            .filter(|route| pending_route_candidate(route))
            .collect::<Vec<_>>();

        let sources = source_rows
            .iter()
            .map(|entry| entry_evidence(entry))
            .collect::<Vec<_>>();
        let targets = target_rows
            .iter()
            .map(|entry| entry_evidence(entry))
            .collect::<Vec<_>>();

        let proof_refs = route_rows
            .iter()
            .map(|route| {
                route
                    .transition
                    .proof_refs
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let registry = route_rows
            .iter()
            .zip(&proof_refs)
            .map(|(route, proof_refs)| RegistryRouteEvidence {
                route_id: &route.route_id,
                matcher: MatcherEvidence {
                    stable_id: route.matcher.stable_id.as_deref(),
                    kind: route.matcher.kind.as_deref(),
                    relative_path: route.matcher.relative_path.as_deref(),
                },
                canonical_target: &route.canonical_target,
                intended_disposition: &route.intended_disposition,
                transition: TransitionEvidence {
                    compatibility_behavior: compatibility_behavior(
                        route.transition.compatibility_behavior,
                    ),
                    compatibility_boundary: compatibility_boundary(
                        route.transition.compatibility_boundary,
                    ),
                    replacement_state: replacement_state(route.transition.replacement_state),
                    active_reader_writer_state: reader_writer_state(
                        route.transition.active_reader_writer_state,
                    ),
                    observed_authority_state: observed_authority_state(
                        route.transition.observed_authority_state,
                    ),
                    equivalence_proof: equivalence_proof(route.transition.equivalence_proof),
                    physical_cleanup_state: physical_cleanup_state(
                        route.transition.physical_cleanup_state,
                    ),
                    proof_refs,
                },
            })
            .collect::<Vec<_>>();

        // Reject unknown, incomplete, conflicting, or mismatched metadata
        // before opening any path selected by an untrusted catalog row.
        let provisional_digests = sources
            .iter()
            .map(|source| DigestEvidence {
                path: source.path,
                sha256: source.sha256,
            })
            .collect::<Vec<_>>();
        let provisional = match verify_catalog(CatalogEvidence {
            raw_sources: &sources,
            raw_targets: &targets,
            raw_registry_routes: &registry,
            current_sources: &provisional_digests,
        }) {
            Ok(catalog) => catalog,
            Err(error) => return Ok(verification_failed(findings, error)),
        };

        let digest_values = provisional
            .routes
            .iter()
            .map(|route| {
                let path = root.join(route.spec.path);
                read_bounded(reads, &path, MAX_PENDING_SOURCE_BYTES)
                    .map(|bytes| (route.spec.path, sha256_hex(&bytes)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let current_digests = digest_values
            .iter()
            .map(|(path, sha256)| DigestEvidence { path, sha256 })
            .collect::<Vec<_>>();

        match verify_catalog(CatalogEvidence {
            raw_sources: &sources,
            raw_targets: &targets,
            raw_registry_routes: &registry,
            current_sources: &current_digests,
        }) {
            Ok(catalog) => Ok(catalog
                .routes
                .into_iter()
                .filter(|route| route.reclassifies_parallel_authority())
                .map(|route| route.spec.stable_id.to_owned())
                .collect()),
            Err(error) => Ok(verification_failed(findings, error)),
        }
    }
}
