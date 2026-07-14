use super::*;

#[cfg(unix)]
pub(crate) fn reject_effective_user_control(
    metadata: &fs::Metadata,
    effective_user_id: libc::uid_t,
) -> Result<(), RoutineError> {
    if effective_user_id == 0 || metadata.uid() == effective_user_id {
        Err(mediator_error("mediator-executable-path-mutable"))
    } else {
        Ok(())
    }
}

pub(crate) struct ScopeAnchor {
    pub(crate) relative: RepoPath,
    pub(crate) path: PathBuf,
    pub(crate) file: File,
    #[cfg(unix)]
    pub(crate) identity: ObjectIdentity,
}

pub(crate) struct OutputConfinement {
    pub(crate) scopes: Vec<ScopeAnchor>,
    pub(crate) budget_bytes: u64,
}

impl OutputConfinement {
    pub(crate) fn prepare(
        root: &RootAnchor,
        scopes: &[RepoPath],
        budget_bytes: u64,
    ) -> Result<Self, RoutineError> {
        #[cfg(not(unix))]
        {
            let _ = (root, scopes, budget_bytes);
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            for (index, left) in scopes.iter().enumerate() {
                if scopes
                    .iter()
                    .skip(index + 1)
                    .any(|right| left.matches_prefix(right) || right.matches_prefix(left))
                {
                    return Err(mediator_error("mediator-output-scope-overlap"));
                }
            }
            let mut anchors = Vec::with_capacity(scopes.len());
            for relative in scopes {
                let path = root.path().join(relative.as_str());
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|_| mediator_error("mediator-output-scope-missing"))?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(mediator_error("mediator-output-scope-not-directory"));
                }
                let canonical = path
                    .canonicalize()
                    .map_err(|_| mediator_error("mediator-output-scope-unavailable"))?;
                if canonical != path || !canonical.starts_with(root.path()) {
                    return Err(mediator_error("mediator-output-scope-escapes-root"));
                }
                let file = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(&path)
                    .map_err(|_| mediator_error("mediator-output-scope-open-failed"))?;
                let identity = ObjectIdentity::from(
                    &file
                        .metadata()
                        .map_err(|_| mediator_error("mediator-output-scope-metadata-failed"))?,
                );
                if identity.device != root.device() {
                    return Err(mediator_error("mediator-output-scope-cross-device"));
                }
                anchors.push(ScopeAnchor {
                    relative: relative.clone(),
                    path,
                    file,
                    identity,
                });
            }
            let confinement = Self {
                scopes: anchors,
                budget_bytes,
            };
            if !confinement.capture()?.is_empty() {
                return Err(mediator_error("mediator-output-scope-not-empty"));
            }
            Ok(confinement)
        }
    }

    pub(crate) fn absolute_scopes(&self) -> Vec<&Path> {
        self.scopes
            .iter()
            .map(|scope| scope.path.as_path())
            .collect()
    }

    pub(crate) fn capture(&self) -> Result<BTreeMap<String, OutputFileRecord>, RoutineError> {
        let mut files = BTreeMap::new();
        let mut consumed = 0_u64;
        for scope in &self.scopes {
            capture_tree(
                &scope.file,
                &scope.relative,
                scope.identity.device,
                self.budget_bytes,
                &mut consumed,
                &mut files,
            )?;
        }
        Ok(files)
    }

    pub(crate) fn capture_owned_delta(
        &self,
    ) -> Result<BTreeMap<String, OutputFileRecord>, RoutineError> {
        self.validate()?;
        let files = self.capture()?;
        if !files.is_empty() {
            return Err(RoutineError::new(
                RoutineErrorId::ConcurrentMutation,
                "mediator-output-scope-mutated",
                None,
            ));
        }
        Ok(BTreeMap::new())
    }

    pub(crate) fn validate(&self) -> Result<(), RoutineError> {
        #[cfg(not(unix))]
        {
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            for scope in &self.scopes {
                let held = ObjectIdentity::from(
                    &scope
                        .file
                        .metadata()
                        .map_err(|_| mediator_error("mediator-output-scope-metadata-failed"))?,
                );
                let current_meta = fs::symlink_metadata(&scope.path)
                    .map_err(|_| mediator_error("mediator-output-scope-replaced"))?;
                if current_meta.file_type().is_symlink() || !current_meta.is_dir() {
                    return Err(mediator_error("mediator-output-scope-replaced"));
                }
                let current = ObjectIdentity::from(&current_meta);
                let canonical = scope
                    .path
                    .canonicalize()
                    .map_err(|_| mediator_error("mediator-output-scope-replaced"))?;
                if held != scope.identity || current != scope.identity || canonical != scope.path {
                    return Err(RoutineError::new(
                        RoutineErrorId::ConcurrentMutation,
                        "mediator-output-scope-replaced",
                        None,
                    ));
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "ownership_rejection_tests.rs"]
mod tests;
