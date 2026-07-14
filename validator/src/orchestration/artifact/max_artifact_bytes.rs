const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRecord {
    pub path: String,
    pub sha256: String,
    pub byte_length: u64,
}

pub(crate) fn validate_records(
    records: &[ArtifactRecord],
    touched: &BTreeSet<CanonicalPath>,
    generated: &BTreeSet<CanonicalPath>,
    fixtures: &BTreeSet<CanonicalPath>,
    lease: &LeaseSpec,
) -> Result<(), OrchestrationError> {
    // Worker results are also validated outside an Orchestrator during review.
    // Reassert the lease invariant before any artifact can use read authority.
    lease.validate_read_write_disjoint()?;
    let mut paths = Vec::with_capacity(records.len());
    for record in records {
        let path = CanonicalPath::parse(&record.path)?;
        if is_host_protected(&path) {
            return Err(OrchestrationError::RootOnlyScope);
        }
        if paths
            .iter()
            .any(|prior: &CanonicalPath| prior.overlaps(&path))
        {
            return Err(OrchestrationError::DuplicateOutput);
        }
        validate_authority(&path, touched, generated, fixtures, lease)?;
        super::model::validate_digest(&record.sha256)?;
        if record.byte_length > MAX_ARTIFACT_BYTES {
            return Err(OrchestrationError::ResourceLimit);
        }
        paths.push(path);
    }
    Ok(())
}

fn validate_authority(
    path: &CanonicalPath,
    touched: &BTreeSet<CanonicalPath>,
    generated: &BTreeSet<CanonicalPath>,
    fixtures: &BTreeSet<CanonicalPath>,
    lease: &LeaseSpec,
) -> Result<(), OrchestrationError> {
    let ordinary = lease.owned_scope.contains_path(path);
    let generated_owned = lease.owned_scope.contains_generated(path);
    let fixture_owned = lease.owned_scope.contains_fixture(path);
    let owned = ordinary || generated_owned || fixture_owned;
    if owned
        && (!touched.contains(path)
            || (generated_owned && !generated.contains(path))
            || (fixture_owned && !fixtures.contains(path)))
    {
        return Err(OrchestrationError::InvalidWorkerResult);
    }
    if !owned
        && (touched.contains(path)
            || lease
                .read_paths
                .iter()
                .all(|root| !path_is_within(path, root)))
    {
        return Err(OrchestrationError::InvalidWorkerResult);
    }
    Ok(())
}

fn is_host_protected(path: &CanonicalPath) -> bool {
    path.as_str().split('/').next().is_some_and(|component| {
        [".git", ".codex", ".agents", ".codex-worktree"]
            .iter()
            .any(|protected| component.eq_ignore_ascii_case(protected))
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedArtifactSet {
    result_id: String,
    artifact_count: usize,
}

impl VerifiedArtifactSet {
    pub fn result_id(&self) -> &str {
        &self.result_id
    }

    pub fn artifact_count(&self) -> usize {
        self.artifact_count
    }
}

#[derive(Clone, Debug)]
pub struct ArtifactWorkspace {
    root: PathBuf,
}

impl ArtifactWorkspace {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, OrchestrationError> {
        let supplied = root.as_ref();
        let metadata =
            fs::symlink_metadata(supplied).map_err(|_| OrchestrationError::InvalidWorkerResult)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(OrchestrationError::InvalidWorkerResult);
        }
        let root =
            fs::canonicalize(supplied).map_err(|_| OrchestrationError::InvalidWorkerResult)?;
        Ok(Self { root })
    }

    pub fn verify(
        &self,
        result: &WorkerResultV1,
        lease: &LeaseSpec,
        package: &WorkPackage,
    ) -> Result<VerifiedArtifactSet, OrchestrationError> {
        self.verify_with(result, lease, package, |_| {})
    }

    fn verify_with<F>(
        &self,
        result: &WorkerResultV1,
        lease: &LeaseSpec,
        package: &WorkPackage,
        mut after_open: F,
    ) -> Result<VerifiedArtifactSet, OrchestrationError>
    where
        F: FnMut(&Path),
    {
        result.validate_for(lease, package)?;
        let mut identities = BTreeSet::new();
        for artifact in &result.artifacts {
            let relative = CanonicalPath::parse(&artifact.path)?;
            let full = self.checked_path(&relative)?;
            let identity = verify_file(&full, artifact, &mut after_open)?;
            if !identities.insert(identity) {
                return Err(OrchestrationError::DuplicateOutput);
            }
        }
        Ok(VerifiedArtifactSet {
            result_id: result.result_id()?,
            artifact_count: result.artifacts.len(),
        })
    }

    #[cfg(test)]
    pub(crate) fn verify_with_hook<F>(
        &self,
        result: &WorkerResultV1,
        lease: &LeaseSpec,
        package: &WorkPackage,
        after_open: F,
    ) -> Result<VerifiedArtifactSet, OrchestrationError>
    where
        F: FnMut(&Path),
    {
        self.verify_with(result, lease, package, after_open)
    }

    fn checked_path(&self, relative: &CanonicalPath) -> Result<PathBuf, OrchestrationError> {
        let mut current = self.root.clone();
        let components: Vec<_> = relative.as_str().split('/').collect();
        for (index, component) in components.iter().enumerate() {
            current.push(component);
            let metadata = fs::symlink_metadata(&current)
                .map_err(|_| OrchestrationError::InvalidWorkerResult)?;
            if metadata.file_type().is_symlink()
                || (index + 1 < components.len() && !metadata.is_dir())
            {
                return Err(OrchestrationError::InvalidWorkerResult);
            }
        }
        let resolved =
            fs::canonicalize(&current).map_err(|_| OrchestrationError::InvalidWorkerResult)?;
        if !resolved.starts_with(&self.root) || resolved != current {
            return Err(OrchestrationError::InvalidWorkerResult);
        }
        Ok(current)
    }
}
