impl ConfinedHostEffectTarget {
    /// Binds one dedicated target root below the same configured temporary
    /// parent as isolated package custody. Opening and observing the root
    /// performs no mutation.
    pub(in crate::distribution::host_effect) fn bind(
        path: &Path,
        scope: AcceptedHostScope,
        target_generation: u64,
    ) -> Result<(Self, ObservedTargetIdentity), HostEffectExecutorFailure> {
        if target_generation == 0 {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::InvalidTargetRoot,
            ));
        }
        let temporary_parent =
            crate::distribution::canonical_temporary_parent().map_err(|_| io_failure())?;
        let canonical = fs::canonicalize(path).map_err(|_| io_failure())?;
        let name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| {
                value.starts_with(ROOT_PREFIX) || value.starts_with(ISOLATED_PACKAGE_ROOT_PREFIX)
            })
            .ok_or_else(|| {
                HostEffectExecutorFailure::new(HostEffectExecutorErrorId::InvalidTargetRoot)
            })?;
        if path != canonical
            || canonical.parent() != Some(temporary_parent.as_path())
            || !valid_component(name)
        {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::InvalidTargetRoot,
            ));
        }
        let parent = open_directory_path(&temporary_parent)?;
        let name = CString::new(name).map_err(|_| io_failure())?;
        let descriptor = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if descriptor < 0 {
            return Err(io_failure());
        }
        let directory = unsafe { File::from_raw_fd(descriptor) };
        let descriptor_metadata = directory.metadata().map_err(|_| io_failure())?;
        let named_metadata = fs::symlink_metadata(&canonical).map_err(|_| io_failure())?;
        let identity = StableDirectoryIdentity::capture(&descriptor_metadata)?;
        if StableDirectoryIdentity::capture(&named_metadata)? != identity {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::TargetSubstitution,
            ));
        }
        let object = HostObjectIdentity::from_metadata(&descriptor_metadata)
            .map_err(|_| target_substitution())?;
        let expected_target = ObservedTargetIdentity::new(&scope, target_generation, object)
            .map_err(|_| target_substitution())?;
        let target = Self {
            anchor: Arc::new(TargetRootAnchor {
                canonical_path: canonical,
                parent,
                directory,
                name,
                identity,
                scope,
                target_generation,
                expected_target: expected_target.clone(),
            }),
        };
        target.revalidate_anchor()?;
        Ok((target, expected_target))
    }

    pub(in crate::distribution::host_effect) fn observer(
        &self,
    ) -> ConfinedHostEffectTargetObserver {
        ConfinedHostEffectTargetObserver {
            target: self.clone(),
        }
    }

    pub(super) fn expected_target(&self) -> &ObservedTargetIdentity {
        &self.anchor.expected_target
    }

    pub(super) fn revalidate_anchor(&self) -> Result<(), HostEffectExecutorFailure> {
        let descriptor = self.anchor.directory.metadata().map_err(|_| path_swap())?;
        if StableDirectoryIdentity::capture(&descriptor)? != self.anchor.identity {
            return Err(path_swap());
        }
        let named = stat_at(
            self.anchor.parent.as_raw_fd(),
            &self.anchor.name,
            libc::AT_SYMLINK_NOFOLLOW,
        )?
        .ok_or_else(path_swap)?;
        if StableDirectoryIdentity::capture(&named)? != self.anchor.identity {
            return Err(path_swap());
        }
        Ok(())
    }

    pub(in crate::distribution::host_effect) fn revalidate_for_recovery(&self) -> bool {
        self.revalidate_anchor().is_ok()
    }

    fn current_target_identity(&self) -> Result<ObservedTargetIdentity, HostEffectExecutorFailure> {
        self.revalidate_anchor()?;
        let metadata = self.anchor.directory.metadata().map_err(|_| path_swap())?;
        let object = HostObjectIdentity::from_metadata(&metadata).map_err(|_| path_swap())?;
        ObservedTargetIdentity::new(&self.anchor.scope, self.anchor.target_generation, object)
            .map_err(|_| target_substitution())
    }

    pub(super) fn prepare(
        &self,
        effect_identity_sha256: &str,
        target_name: &str,
        bytes: Vec<u8>,
    ) -> Result<PreparedPublication, HostEffectExecutorFailure> {
        if !valid_component(target_name)
            || !target_name.ends_with(".json")
            || bytes.is_empty()
            || bytes.len() > MAX_PUBLICATION_BYTES
        {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::UnsafeObject,
            ));
        }
        self.require_clean_publication_name(target_name)?;
        let target = self.observe_object(target_name, 1, false)?;
        if target.kind != PublicationObjectKind::Missing {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::Replay,
            ));
        }
        let mut nonce = [0_u8; 8];
        getrandom::fill(&mut nonce).map_err(|_| io_failure())?;
        let temporary_name = format!(
            ".{target_name}.{}.tmp",
            nonce
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        nonce.fill(0);
        if self.observe_object(&temporary_name, 1, false)?.kind != PublicationObjectKind::Missing {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::TempCollision,
            ));
        }
        let content_sha256 = digest_bytes(&bytes);
        let mode = u32::from(libc::S_IFREG) | PUBLICATION_MODE;
        let prior = ExpectedPublicationObjectIdentity::missing(target_name.to_owned())
            .map_err(|_| unsafe_object())?;
        let next = ExpectedPublicationObjectIdentity::regular(ExpectedRegularPublicationObject {
            name: target_name.to_owned(),
            byte_length: bytes.len() as u64,
            mode,
            hard_links: 1,
            content_sha256: content_sha256.clone(),
            data_synced: true,
        })
        .map_err(|_| unsafe_object())?;
        let temporary =
            ExpectedPublicationObjectIdentity::regular(ExpectedRegularPublicationObject {
                name: temporary_name.clone(),
                byte_length: bytes.len() as u64,
                mode,
                hard_links: 1,
                content_sha256,
                data_synced: true,
            })
            .map_err(|_| unsafe_object())?;
        let expectation =
            PublicationExpectation::new(effect_identity_sha256.to_owned(), prior, next, temporary)
                .map_err(|_| unsafe_object())?;
        Ok(PreparedPublication {
            target_name: target_name.to_owned(),
            temporary_name,
            bytes,
            expectation,
        })
    }
}
