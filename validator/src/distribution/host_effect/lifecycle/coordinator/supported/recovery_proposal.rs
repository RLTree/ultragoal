impl<'a> SupportedHostLifecycleCoordinator<'a> {
    pub(super) fn recovery_proposal(
        &self,
        classification: &PublicationClassification,
        authorization: RecoveryAuthorization,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<RecoveryProposal, SupportedHostLifecycleError> {
        let time = clock.sample()?;
        let head = self.ledger.ledger.head().map_err(|_| ledger_rejected())?;
        propose_recovery(
            classification,
            authorization,
            &self.binding_sha256()?,
            &head,
            time.unix_ms(),
        )
    }

    fn binding_sha256(&self) -> Result<String, SupportedHostLifecycleError> {
        if self.authority.ledger_id != self.ledger.ledger_id {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::CoordinatorSubstitution,
            ));
        }
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            issuer_id: &'a str,
            ledger_id: &'a str,
            authority_binding_sha256: &'a str,
            ledger_binding_sha256: &'a str,
        }
        digest_json(&Binding {
            schema: "harness-ultragoal.supported-host-lifecycle-coordinator.v1",
            issuer_id: &self.authority.issuer_id,
            ledger_id: &self.authority.ledger_id,
            authority_binding_sha256: &self.authority.binding_sha256,
            ledger_binding_sha256: &self.ledger.binding_sha256,
        })
    }

    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn prepare_as_platform(
        &self,
        platform: DescriptorExecutionPlatform,
        request: HostEffectPreparationRequest<'_>,
    ) -> Result<DescriptorExecutionHandoff, SupportedHostLifecycleError> {
        self.prepare_for_platform(platform, request)
    }
}

pub(crate) struct DescriptorExecutionHandoff {
    capability: DescriptorExecutionCapability,
    effect: AuthorizedHostEffect,
    target: Box<dyn HostTargetLease>,
}

impl std::fmt::Debug for DescriptorExecutionHandoff {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DescriptorExecutionHandoff")
            .field("platform", &self.capability.platform)
            .field("primitive", &self.capability.primitive)
            .finish_non_exhaustive()
    }
}

impl DescriptorExecutionHandoff {
    /// Keeps the target lease owned by the opaque handoff for the complete
    /// synchronous adapter call. Neither effect authority nor the lease can
    /// escape as an owned value.
    pub(in crate::distribution::host_effect) fn with_retained_authority<R>(
        mut self,
        adapter: impl FnOnce(
            &DescriptorExecutionCapability,
            &AuthorizedHostEffect,
            &mut dyn HostTargetLease,
        ) -> R,
    ) -> R {
        adapter(&self.capability, &self.effect, self.target.as_mut())
    }
}

fn recompute_time_attestation(
    sample: &TrustedTimeSample,
) -> Result<String, SupportedHostLifecycleError> {
    #[derive(Serialize)]
    struct Attestation<'a> {
        schema: &'static str,
        source_name: &'a str,
        source_epoch: u64,
        sequence: u64,
        unix_ms: u64,
    }
    digest_json(&Attestation {
        schema: "harness-ultragoal.trusted-time-sample.v1",
        source_name: &sample.source_name,
        source_epoch: sample.source_epoch,
        sequence: sample.sequence,
        unix_ms: sample.unix_ms,
    })
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn digest_json(value: &impl Serialize) -> Result<String, SupportedHostLifecycleError> {
    serde_json::to_vec(value)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| invalid())
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn invalid() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::InvalidAcceptedIdentity)
}

fn untrusted_time() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::UntrustedTime)
}

fn authority_rejected() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::AuthorityRejected)
}

fn ledger_rejected() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::LedgerRejected)
}
