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
