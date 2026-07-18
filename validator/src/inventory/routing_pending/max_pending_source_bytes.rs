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
        ReplacementState::NotApplicable => "not-applicable",
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
