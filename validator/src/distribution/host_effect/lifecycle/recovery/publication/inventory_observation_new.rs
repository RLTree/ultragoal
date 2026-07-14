impl PublicationInventoryObservation {
    pub(in crate::distribution::host_effect) fn new(
        request: PublicationInventoryObservationRequest,
    ) -> Result<Self, SupportedHostLifecycleError> {
        let PublicationInventoryObservationRequest {
            scan_generation_before,
            scan_generation_after,
            target,
            temporary_objects,
            expectation,
            current_ledger_head,
            acknowledgement,
        } = request;
        if scan_generation_before == 0
            || scan_generation_after == 0
            || target.name != expectation.next.name
            || temporary_objects
                .iter()
                .any(|object| !temporary_name(&object.name, &target.name))
        {
            return Err(recovery_unsafe());
        }
        Ok(Self {
            scan_generation_before,
            scan_generation_after,
            target,
            temporary_objects,
            expectation,
            current_ledger_head,
            acknowledgement,
        })
    }

    pub(in crate::distribution::host_effect) fn classify(
        &self,
    ) -> Result<PublicationClassification, SupportedHostLifecycleError> {
        classify_publication(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PublicationClassificationId {
    CleanPriorState,
    InterruptedBeforeTempWrite,
    InterruptedDuringTempFsync,
    InterruptedBeforeRename,
    CommittedBeforeAcknowledgement,
    AcknowledgedCommitted,
    OrphanedTemporaryObject,
    MultipleTemporaryObjects,
    UnsafeSpecialObject,
    ObservationRace,
    FalsePassReceipt,
    UnknownState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PublicationClassification {
    id: PublicationClassificationId,
    evidence_sha256: String,
    temporary_object: Option<String>,
}

impl PublicationClassification {
    pub(crate) const fn id(&self) -> PublicationClassificationId {
        self.id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RecoveryProposalAction {
    ResumeWithoutMutation,
    QuarantineTemporaryObjectForReview,
    ReconcileCommittedStateBeforeAcknowledgement,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RecoveryProposal {
    action: RecoveryProposalAction,
    classification_sha256: String,
    proposal_sha256: String,
    automatic_cleanup: bool,
}

impl RecoveryProposal {
    pub(crate) const fn action(&self) -> RecoveryProposalAction {
        self.action
    }

    pub(crate) const fn automatic_cleanup(&self) -> bool {
        self.automatic_cleanup
    }
}

pub(super) struct RecoveryAuthorization {
    schema_version: String,
    classification_sha256: String,
    coordinator_binding_sha256: String,
    ledger_head: HostEffectLedgerHead,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    nonce_sha256: String,
    authorization_sha256: String,
}

pub(super) fn issue_recovery_authorization(
    classification: &PublicationClassification,
    coordinator_binding_sha256: &str,
    head: &HostEffectLedgerHead,
    issued_at_unix_ms: u64,
) -> Result<RecoveryAuthorization, SupportedHostLifecycleError> {
    if issued_at_unix_ms == 0 {
        return Err(recovery_authorization_required());
    }
    let expires_at_unix_ms = issued_at_unix_ms
        .checked_add(60_000)
        .ok_or_else(recovery_authorization_required)?;
    let mut nonce = [0_u8; 32];
    getrandom::fill(&mut nonce).map_err(|_| recovery_authorization_required())?;
    let nonce_sha256 = digest_bytes(&nonce);
    nonce.fill(0);
    let authorization_sha256 = authorization_digest(
        RECOVERY_AUTHORIZATION_SCHEMA,
        &classification.evidence_sha256,
        coordinator_binding_sha256,
        head,
        issued_at_unix_ms,
        expires_at_unix_ms,
        &nonce_sha256,
    )?;
    Ok(RecoveryAuthorization {
        schema_version: RECOVERY_AUTHORIZATION_SCHEMA.to_owned(),
        classification_sha256: classification.evidence_sha256.clone(),
        coordinator_binding_sha256: coordinator_binding_sha256.to_owned(),
        ledger_head: head.clone(),
        issued_at_unix_ms,
        expires_at_unix_ms,
        nonce_sha256,
        authorization_sha256,
    })
}
