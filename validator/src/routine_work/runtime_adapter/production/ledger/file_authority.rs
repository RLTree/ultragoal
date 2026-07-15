use super::*;

impl FileAuthorityLedger {
    pub(crate) fn open_or_initialize(root: &Path) -> Result<Self, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return supported::FileLedger::open_or_initialize(root).map(|inner| Self { inner });
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = root;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    /// Opens only an already-complete authority store. Unlike
    /// `open_or_initialize`, this path never creates the lock, key, or state and
    /// is therefore safe for untrusted supplied-reuse authentication.
    pub(crate) fn open_existing(root: &Path) -> Result<Self, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return supported::FileLedger::open_existing(root).map(|inner| Self { inner });
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = root;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn preauthorize_reuse(
        &self,
        binding: &AuthorityBinding,
        claims: Vec<ReuseArtifactClaim>,
    ) -> Result<ReusePreauthorization, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.preauthorize_reuse(binding, claims);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (binding, claims);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn reserve(&self, spec: ReservationSpec) -> Result<ReservationToken, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.reserve(spec);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = spec;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn validate_reserved(&self, token: &ReservationToken) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.validate_reserved(token);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = token;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn prepare_spawn(&self, token: &ReservationToken) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.prepare_spawn(token);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = token;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn record_output_component(
        &self,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
    ) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self
                .inner
                .record_output_component(token, relative_path, identity);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, relative_path, identity);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn record_output_staged(
        &self,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
    ) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self
                .inner
                .record_output_staged(token, relative_path, identity);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, relative_path, identity);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn stage_success(
        &self,
        token: &ReservationToken,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.stage_success(token, artifacts);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, artifacts);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn settle(
        &self,
        token: &ReservationToken,
        state: AttemptState,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.settle(token, state, artifacts);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, state, artifacts);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn authenticates(
        &self,
        token: &ReservationToken,
        digest: &str,
        witness: &str,
    ) -> Result<bool, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.authenticates(token, digest, witness);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, digest, witness);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn pending_recovery(
        &self,
        binding: &AuthorityBinding,
    ) -> Result<Option<PendingRecovery>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.pending_recovery(binding);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = binding;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    #[cfg(test)]
    pub(crate) fn test_expire_pending(
        &self,
        binding: &AuthorityBinding,
    ) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.test_expire_pending(binding);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = binding;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    #[cfg(all(test, target_vendor = "apple"))]
    pub(crate) fn test_expire_reservation(
        &self,
        token: &mut ReservationToken,
    ) -> Result<(), RoutineError> {
        let expired = self.inner.test_expire_reservation(token)?;
        token.expires_tick = expired;
        Ok(())
    }
}

pub(crate) fn error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
