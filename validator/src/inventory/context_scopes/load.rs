pub(super) fn load(
    reads: &ReadSession,
    root: &Path,
    current_contract_id: &str,
) -> Result<ContextScopes, InventoryError> {
    let registry_path = root.join(REGISTRY_RELATIVE);
    let candidate_path = root.join(CANDIDATE_ROOT);
    let has_registry = exists(&registry_path)?;
    let has_candidate = exists(&candidate_path)?;
    if !has_registry && !has_candidate {
        reads
            .observe_presence_parent(&registry_path)
            .map_err(|error| InventoryError::Io {
                path: registry_path,
                message: error.to_string(),
            })?;
        return Ok(ContextScopes::empty());
    }
    if !has_registry {
        return Err(InventoryError::InvalidRegistry(format!(
            "{REGISTRY_RELATIVE} is required while {CANDIDATE_ROOT} exists"
        )));
    }
    let bytes = match read_bounded(reads, &registry_path, MAX_REGISTRY_BYTES) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Ok(ContextScopes::rejected(
                "invalid_non_authoritative_context_registry",
                REGISTRY_RELATIVE,
                "the context registry is not a bounded regular file",
            ));
        }
    };
    let parsed = super::plugin_manifest_json::unique_keys(&bytes)
        .then(|| serde_json::from_slice::<Registry>(&bytes).ok())
        .flatten();
    let Some(registry) = parsed
        .as_ref()
        .filter(|registry| supported_registry(registry, current_contract_id))
    else {
        return Ok(ContextScopes::rejected(
            "invalid_non_authoritative_context_registry",
            REGISTRY_RELATIVE,
            "the context registry does not exactly bind the supported candidate scope",
        ));
    };
    let candidate_files = match manifest::verify(reads, root) {
        Ok(files) => files
            .into_iter()
            .map(|file| format!("{CANDIDATE_ROOT}/{file}"))
            .collect(),
        Err(_) => {
            return Ok(ContextScopes::rejected(
                "non_authoritative_context_verification_failed",
                CANDIDATE_ROOT,
                "the candidate context bundle failed exact manifest verification",
            ));
        }
    };
    let mut scoped = vec![(
        CANDIDATE_CONTEXT_ID,
        "non-authoritative-contract-context",
        candidate_files,
        vec![
            REGISTRY_RELATIVE.to_owned(),
            format!("{CANDIDATE_ROOT}/ZIP_INCLUDE_MANIFEST.json"),
        ],
    )];
    if registry.schema_version == SCHEMA_VERSION_V2 {
        let predecessor = match exact_set::verify(reads, root, &registry.exact_contexts[0]) {
            Ok(files) => files,
            Err(_) => {
                return Ok(ContextScopes::rejected(
                    "non_authoritative_context_verification_failed",
                    REGISTRY_RELATIVE,
                    "the predecessor contract context failed exact-set verification",
                ));
            }
        };
        scoped.push((
            exact_set::context_id(),
            "non-authoritative-contract-context",
            predecessor,
            vec![REGISTRY_RELATIVE.to_owned()],
        ));
        let worker_results = match evidence::verify(reads, root, &registry.evidence_contexts[0]) {
            Ok(files) => files,
            Err(_) => {
                return Ok(ContextScopes::rejected(
                    "non_authoritative_context_verification_failed",
                    REGISTRY_RELATIVE,
                    "the worker evidence context failed path and record verification",
                ));
            }
        };
        scoped.push((
            evidence::context_id(),
            "run-scoped-worker-evidence-context",
            worker_results,
            vec![
                REGISTRY_RELATIVE.to_owned(),
                evidence::schema_ref().to_owned(),
            ],
        ));
        if let Some(row) = registry.proposal_contexts.first() {
            let proposal = match proposal_context::verify(reads, root, row) {
                Ok(path) => vec![path],
                Err(_) => {
                    return Ok(ContextScopes::rejected(
                        "non_authoritative_context_verification_failed",
                        REGISTRY_RELATIVE,
                        "the predecessor proposal context failed exact-file verification",
                    ));
                }
            };
            scoped.push((
                proposal_context::context_id(),
                "non-authoritative-proposal-context",
                proposal,
                vec![REGISTRY_RELATIVE.to_owned()],
            ));
        }
    }
    let mut result = ContextScopes::empty();
    result.entries.push(physical_regular_entry(
        reads,
        root,
        &registry_path,
        PhysicalEntryDescriptor {
            stable_id: "CONTEXT-SCOPE-REGISTRY".to_owned(),
            kind: "context-scope-registry",
            owner: "OWN-ULTRA-ROOT",
            authority_state: AuthorityState::Canonical,
            active_status: ActiveStatus::Active,
            generator: None,
            provenance: Vec::new(),
            references: vec![format!("context-scope:{CANDIDATE_CONTEXT_ID}")],
        },
    )?);
    for (context_id, kind, files, proof_refs) in scoped {
        for relative in files {
            result.entries.push(context_entry(
                reads,
                root,
                &relative,
                context_id,
                kind,
                proof_refs.clone(),
            )?);
            result.verified_paths.insert(relative);
        }
    }
    Ok(result)
}
