impl AcceptedHostScope {
    pub(in crate::distribution::host_effect) fn binding_sha256(
        &self,
    ) -> Result<String, SupportedHostLifecycleError> {
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            scope: &'a AcceptedHostScope,
        }
        digest_json(&Binding {
            schema: "harness-ultragoal.accepted-host-scope.v1",
            scope: self,
        })
    }

    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn personal(
        journey: &JourneyBinding,
        marketplace: String,
    ) -> Result<Self, SupportedHostLifecycleError> {
        validate_name(&marketplace)?;
        if marketplace != journey.marketplace() {
            return Err(invalid());
        }
        Ok(Self::Personal {
            home_id: journey.home_id().to_owned(),
            host_id: journey.host_id().to_owned(),
            marketplace,
        })
    }

    pub(in crate::distribution::host_effect) fn repository(
        journey: &JourneyBinding,
        marketplace: String,
    ) -> Result<Self, SupportedHostLifecycleError> {
        validate_name(&marketplace)?;
        if marketplace != journey.marketplace() {
            return Err(invalid());
        }
        Ok(Self::Repository {
            home_id: journey.home_id().to_owned(),
            project_id: journey.project_id().to_owned(),
            host_id: journey.host_id().to_owned(),
            marketplace,
        })
    }

    fn validate_for(&self, journey: &JourneyBinding) -> Result<(), SupportedHostLifecycleError> {
        let (home_id, project_id, host_id, marketplace) = match self {
            #[cfg(test)]
            Self::Personal {
                home_id,
                host_id,
                marketplace,
            } => (home_id, None, host_id, marketplace),
            Self::Repository {
                home_id,
                project_id,
                host_id,
                marketplace,
            } => (home_id, Some(project_id), host_id, marketplace),
        };
        validate_name(marketplace)?;
        if home_id != journey.home_id()
            || host_id != journey.host_id()
            || marketplace != journey.marketplace()
            || project_id.is_some_and(|value| value != journey.project_id())
        {
            return Err(invalid());
        }
        Ok(())
    }

    fn required_capabilities(&self, operation: AcceptedLifecycleOperation) -> Vec<Capability> {
        if !operation.is_effectful() {
            return Vec::new();
        }
        let mut required = vec![Capability::Install, Capability::Marketplace];
        if matches!(self, Self::Repository { .. }) {
            required.insert(0, Capability::Filesystem);
        }
        required
    }

    fn validate_plan(
        &self,
        package: &PackageIdentity,
        operation: AcceptedLifecycleOperation,
        plan: &HostCommandPlan,
    ) -> Result<(), SupportedHostLifecycleError> {
        let remove = operation == AcceptedLifecycleOperation::UninstallTeardown;
        let expected = match self {
            #[cfg(test)]
            Self::Personal { marketplace, .. } if remove => {
                HostCommandPlan::personal_remove(package, marketplace)
            }
            #[cfg(test)]
            Self::Personal { marketplace, .. } => {
                HostCommandPlan::personal_install(package, marketplace)
            }
            Self::Repository { marketplace, .. } if remove => {
                HostCommandPlan::repository_remove(package, marketplace)
            }
            Self::Repository {
                project_id,
                marketplace,
                ..
            } => {
                let repository_root = plan
                    .commands()
                    .first()
                    .filter(|command| {
                        command.program() == "codex"
                            && command.argv().len() == 4
                            && command.argv()[..3] == ["plugin", "marketplace", "add"]
                    })
                    .map(|command| command.argv()[3].as_str())
                    .ok_or_else(invalid)?;
                let isolated_home = plan
                    .commands()
                    .first()
                    .and_then(|command| command.environment().iter().find(|(key, _)| key == "HOME"))
                    .map(|(_, value)| Path::new(value))
                    .ok_or_else(invalid)?;
                if project_identity(repository_root)? != *project_id {
                    return Err(invalid());
                }
                HostCommandPlan::repository_install_in_isolated_codex_home(
                    package,
                    repository_root,
                    marketplace,
                    isolated_home,
                )
            }
        }
        .map_err(|_| invalid())?;
        if expected.plan_sha256() != plan.plan_sha256() {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::PlanSubstitution,
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostObjectIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    hard_links: u64,
    byte_length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl HostObjectIdentity {
    #[cfg(unix)]
    pub(in crate::distribution::host_effect) fn from_metadata(
        metadata: &Metadata,
    ) -> Result<Self, SupportedHostLifecycleError> {
        use std::os::unix::fs::MetadataExt;
        if !metadata.is_dir() && !metadata.is_file() {
            return Err(invalid());
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            hard_links: metadata.nlink(),
            byte_length: metadata.size(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        })
    }

    #[cfg(not(unix))]
    pub(in crate::distribution::host_effect) fn from_metadata(
        _metadata: &Metadata,
    ) -> Result<Self, SupportedHostLifecycleError> {
        Err(invalid())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ObservedTargetIdentity {
    scope_sha256: String,
    generation: u64,
    object: HostObjectIdentity,
    target_sha256: String,
}
