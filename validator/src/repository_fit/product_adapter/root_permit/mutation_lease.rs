use super::*;

/// Effect capability moved into exactly one mediation attempt. No constructor
/// exists in production until root-owned LocalEffects activation is wired.
#[must_use = "an exclusive repository-fit mutation lease must be consumed once"]
pub(crate) struct RepositoryFitMutationLease<E: RepositoryFitPermitEffects> {
    pub(crate) binding_id: String,
    pub(crate) authority: Arc<AuthorityIdentity>,
    pub(crate) seal: Arc<ApplyRequestSeal>,
    pub(crate) effects: ScopedEffects<E>,
}

impl<E: RepositoryFitPermitEffects> Debug for RepositoryFitMutationLease<E> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RepositoryFitMutationLease")
            .field("binding", &"[bound]")
            .field("authority", &"[redacted]")
            .finish()
    }
}

pub(crate) trait RepositoryFitPermitEffects: FitEffects {}

#[cfg(unix)]
impl RepositoryFitPermitEffects for LocalEffects {}

pub(crate) struct ScopedEffects<E> {
    pub(crate) inner: E,
    pub(crate) allowed: Vec<AllowedMutation>,
    pub(crate) root: PathBuf,
    pub(crate) target_paths: Vec<CanonicalPath>,
    pub(crate) target_prestate: TargetSnapshot,
    pub(crate) authorized_target: TargetCapture,
    pub(crate) scope_violation: bool,
    pub(crate) chain_violation: bool,
}

#[derive(Clone)]
pub(crate) struct AllowedMutation {
    pub(crate) path: CanonicalPath,
    pub(crate) forward_expected: ExpectedContent,
    pub(crate) replacement: Vec<u8>,
    pub(crate) prior: Option<Vec<u8>>,
    pub(crate) forward_mode: u32,
    pub(crate) prior_mode: Option<u32>,
}

impl<E> ScopedEffects<E> {
    pub(crate) fn new(
        inner: E,
        request: &OpaqueFitApplyRequest,
        root: &Path,
        target_prestate: TargetSnapshot,
        authorized_target: TargetCapture,
    ) -> Self {
        let allowed = request
            .all_mutations()
            .iter()
            .map(|mutation| AllowedMutation {
                path: mutation.path.clone(),
                forward_expected: mutation.expected.clone(),
                replacement: mutation.replacement.clone(),
                prior: mutation.prior.clone(),
                forward_mode: request.unix_modes[mutation.path.as_str()],
                prior_mode: request.observed_modes[mutation.path.as_str()],
            })
            .collect();
        Self {
            inner,
            allowed,
            root: root.to_path_buf(),
            target_paths: request.target_paths(),
            target_prestate,
            authorized_target,
            scope_violation: false,
            chain_violation: false,
        }
    }

    pub(crate) fn scope_violation(&self) -> bool {
        self.scope_violation || self.chain_violation
    }

    pub(crate) fn capture_authorized(&mut self) -> Result<TargetCapture, FitError> {
        if self.authorized_target.chain.revalidate().is_err() {
            self.chain_violation = true;
            return Err(chain_fit_error());
        }
        let current = capture_target_descriptor_chain_for_paths(&self.root, &self.target_paths)
            .map_err(|_| chain_fit_error())?;
        if current.snapshot != self.authorized_target.snapshot {
            self.chain_violation = true;
            return Err(chain_fit_error());
        }
        Ok(current)
    }

    pub(crate) fn revalidate_authorized_target(
        &mut self,
    ) -> Result<TargetSnapshot, FitAdapterError> {
        self.capture_authorized()
            .map(|capture| capture.snapshot)
            .map_err(|_| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))
    }
}

pub(crate) fn chain_fit_error() -> FitError {
    crate::repository_fit::error(FitErrorId::StaleBinding)
}

impl<E: RepositoryFitPermitEffects> FitReader for ScopedEffects<E> {
    fn root_binding(&mut self) -> Result<String, FitError> {
        self.inner.root_binding()
    }

    fn read_file(
        &mut self,
        path: &CanonicalPath,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        self.inner.read_file(path, maximum_bytes)
    }
}
