use super::*;

impl LiveContext {
    pub fn build(request: BuildRequest) -> Result<Self, ContextError> {
        let grant = request.root_grant.as_ref();
        if request.effect != EffectClass::Read
            && grant.is_none_or(|grant| grant.effect() != request.effect)
        {
            return Err(ContextError::EffectDenied(format!(
                "selected {:?} effect lacks a matching root-issued grant",
                request.effect
            )));
        }
        let git_path = capability::resolve_git()?;
        let git_before = capability::executable_identity("git", &git_path)?;
        let (repository, worktree) = git::resolve_roots(&request.start, &git_path)?;
        match_expected(
            "repository",
            request.expected_repository_root.as_ref(),
            &repository,
        )?;
        match_expected(
            "worktree",
            request.expected_worktree_root.as_ref(),
            &worktree,
        )?;
        let scopes = canonical_scopes(
            grant.map(|grant| grant.write_scopes()).unwrap_or_default(),
            &worktree,
        )?;
        if matches!(
            request.effect,
            EffectClass::PlannedWrite | EffectClass::WorkspaceWrite | EffectClass::Destructive
        ) && scopes.is_empty()
        {
            return Err(ContextError::EffectDenied(
                "workspace-affecting context requires an explicit write scope".to_owned(),
            ));
        }
        let candidate = git::capture_candidate(&git_path, &worktree)?;
        let inputs = selected_inputs(&request.selected_inputs, &worktree)?;
        let capabilities = capability::capture(&request.tool_probes, &git_path)?;
        let permission_identity = permissions(&repository, &worktree);
        if candidate != git::capture_candidate(&git_path, &worktree)? {
            return Err(ContextError::ConcurrentMutation(
                "Git candidate identity".to_owned(),
            ));
        }
        if inputs != selected_inputs(&request.selected_inputs, &worktree)? {
            return Err(ContextError::ConcurrentMutation(
                "selected inputs".to_owned(),
            ));
        }
        if capabilities != capability::capture(&request.tool_probes, &git_path)?
            || git_before != capability::executable_identity("git", &git_path)?
        {
            return Err(ContextError::ConcurrentMutation(
                "tool capabilities, PATH, or Git substrate".to_owned(),
            ));
        }
        if permission_identity != permissions(&repository, &worktree) {
            return Err(ContextError::ConcurrentMutation(
                "root permissions".to_owned(),
            ));
        }
        let roots = RootIdentity {
            repository_root: path_text(&repository)?,
            worktree_root: path_text(&worktree)?,
        };
        let permitted = permitted_effects(request.effect);
        let payload = ContextPayload::new(
            roots,
            candidate,
            configuration::identity(&request.configuration, &request.secret_sources)?,
            capabilities,
            permission_identity,
            EffectBoundary {
                selected: request.effect,
                permitted,
                write_scopes: scopes,
            },
            inputs,
        );
        let serialized = serde_json::to_vec(&payload)
            .map_err(|error| ContextError::Serialization(error.to_string()))?;
        let context = Self::from_payload(payload, format!("sha256:{}", sha256_hex(&serialized)));
        context.revalidate()?;
        Ok(context)
    }

    pub fn to_canonical_json(&self) -> Result<Vec<u8>, ContextError> {
        serde_json::to_vec(self).map_err(|error| ContextError::Serialization(error.to_string()))
    }
}
