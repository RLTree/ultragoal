use super::*;

impl FixtureCaptureAdapter {
    pub(crate) fn issue(
        fixture: &FixtureSpec,
        executable: PathBuf,
        arguments: Vec<OsString>,
        output_limit: usize,
        required_output: Vec<u8>,
    ) -> Result<Self, FixtureScheduleError> {
        let binding = FixtureExecutionBinding::standalone(&fixture.id, &fixture.metadata_digest);
        Self::issue_bound(
            fixture,
            FixtureCaptureRequest {
                executable,
                arguments,
                output_limit,
                required_output,
                binding,
            },
            None,
            ExecutableIssuancePolicy::Production,
        )
    }

    #[cfg(test)]
    pub(crate) fn issue_test_native(
        fixture: &FixtureSpec,
        executable: PathBuf,
        arguments: Vec<OsString>,
        output_limit: usize,
        required_output: Vec<u8>,
    ) -> Result<Self, FixtureScheduleError> {
        let binding = FixtureExecutionBinding::standalone(&fixture.id, &fixture.metadata_digest);
        Self::issue_bound(
            fixture,
            FixtureCaptureRequest {
                executable,
                arguments,
                output_limit,
                required_output,
                binding,
            },
            None,
            ExecutableIssuancePolicy::TestNativeSnapshot,
        )
    }

    pub(crate) fn issue_evaluation(
        fixture: &FixtureSpec,
        request: FixtureCaptureRequest,
        artifact_name: impl Into<String>,
    ) -> Result<Self, FixtureScheduleError> {
        let artifact_name = artifact_name.into();
        if artifact_name.is_empty()
            || artifact_name.len() > 120
            || artifact_name == "."
            || artifact_name == ".."
            || artifact_name.contains('/')
            || artifact_name.contains('\\')
            || artifact_name.chars().any(char::is_control)
            || !fixture.resources.contains(&ResourceKind::File)
        {
            return Err(FixtureScheduleError::InvalidMetadata(
                "invalid evaluation artifact name".to_owned(),
            ));
        }
        Self::issue_bound(
            fixture,
            request,
            Some(artifact_name),
            ExecutableIssuancePolicy::Production,
        )
    }

    pub(crate) fn issue_bound(
        fixture: &FixtureSpec,
        request: FixtureCaptureRequest,
        artifact_name: Option<String>,
        issuance_policy: ExecutableIssuancePolicy,
    ) -> Result<Self, FixtureScheduleError> {
        fixture.validate()?;
        let FixtureCaptureRequest {
            executable,
            arguments,
            output_limit,
            required_output,
            binding,
        } = request;
        if !executable.is_absolute()
            || executable
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
            || !(1..=MAX_OUTPUT).contains(&output_limit)
            || required_output.len() > MAX_OUTPUT
        {
            return Err(FixtureScheduleError::InvalidMetadata(
                "invalid fixture capture permit".to_owned(),
            ));
        }
        #[cfg(not(unix))]
        {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable pinning requires Unix".to_owned(),
            ));
        }
        #[cfg(unix)]
        {
            let (executable_bytes, executable_digest, executable_identity, executable_kind) =
                pin_source_executable(&executable, issuance_policy)?;
            Ok(Self {
                fixture_id: fixture.id.clone(),
                fixture_digest: fixture.metadata_digest.clone(),
                executable,
                executable_digest,
                executable_bytes: Arc::from(executable_bytes),
                executable_kind,
                executable_identity,
                arguments,
                output_limit,
                interrupt: Arc::new(AtomicBool::new(false)),
                required_output,
                binding,
                artifact_name,
            })
        }
    }

    pub(crate) fn execution_bytes(&self) -> Result<Arc<[u8]>, FixtureScheduleError> {
        if crate::digest::bytes(&self.executable_bytes) != self.executable_digest {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable snapshot binding changed before launch".to_owned(),
            ));
        }
        Ok(Arc::clone(&self.executable_bytes))
    }

    #[cfg(unix)]
    pub(crate) fn validate_protected_native(&self) -> Result<(), FixtureScheduleError> {
        if self.executable_kind != PinnedExecutableKind::ProtectedNative {
            return Err(FixtureScheduleError::Integrity(
                "fixture executable is not a protected native substrate".to_owned(),
            ));
        }
        require_protected_native_path(&self.executable)?;
        let path_metadata = std::fs::symlink_metadata(&self.executable)?;
        require_safe_source(&path_metadata)?;
        if ExecutableIdentity::from(&path_metadata).user_id != 0
            || ExecutableIdentity::from(&path_metadata) != self.executable_identity
        {
            return Err(FixtureScheduleError::Integrity(
                "protected fixture executable identity changed".to_owned(),
            ));
        }
        let file = open_source_descriptor(&self.executable)?;
        if ExecutableIdentity::from(&file.metadata()?) != self.executable_identity
            || crate::digest::bytes(&read_exact_descriptor(&file, MAX_EXECUTABLE_BYTES)?)
                != self.executable_digest
        {
            return Err(FixtureScheduleError::Integrity(
                "protected fixture executable content changed".to_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(unix)]
    pub(crate) fn validate_posix_shell_substrate() -> Result<(), FixtureScheduleError> {
        let (_, _, identity, kind) = pin_source_executable(
            Path::new(POSIX_SHELL_PATH),
            ExecutableIssuancePolicy::ShellSubstrate,
        )?;
        if identity.user_id != 0 || kind != PinnedExecutableKind::ProtectedNative {
            return Err(FixtureScheduleError::Integrity(
                "fixed POSIX shell substrate is not protected".to_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn set_test_issue_pause(path: PathBuf, milliseconds: u64) {
        TEST_ISSUE_PAUSED.store(false, Ordering::SeqCst);
        *issue_hook().lock().expect("fixture issue hook lock") = Some((path, milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_issue_is_paused() -> bool {
        TEST_ISSUE_PAUSED.load(Ordering::SeqCst)
    }

    pub(crate) fn interrupt(&self) {
        self.interrupt.store(true, Ordering::SeqCst);
    }

    #[cfg(unix)]
    pub(crate) fn executable_identity_sha256(&self) -> String {
        crate::fixture_scheduler::stable_digest(&format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.executable_identity.device,
            self.executable_identity.inode,
            self.executable_identity.length,
            self.executable_identity.link_count,
            self.executable_identity.user_id,
            self.executable_identity.group_id,
            self.executable_identity.mode,
            self.executable_identity.modified_seconds,
            self.executable_identity.modified_nanos,
            self.executable_identity.changed_seconds,
            self.executable_identity.changed_nanos,
        ))
    }

    #[cfg(not(unix))]
    pub(crate) fn executable_identity_sha256(&self) -> String {
        self.executable_digest.clone()
    }
}

#[cfg(unix)]
impl ExecutableIdentity {
    pub(crate) fn from(metadata: &std::fs::Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            link_count: metadata.nlink(),
            user_id: metadata.uid(),
            group_id: metadata.gid(),
            mode: metadata.mode(),
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
        }
    }
}
