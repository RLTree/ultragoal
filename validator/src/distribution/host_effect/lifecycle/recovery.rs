use super::super::HostEffectLedgerHead;
use super::{SupportedHostLifecycleError, SupportedHostLifecycleErrorId, lifecycle_error};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const RECOVERY_AUTHORIZATION_SCHEMA: &str =
    "harness-ultragoal.host-effect-recovery-authorization.v2";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PublicationObjectKind {
    Missing,
    Regular,
    Symlink,
    Directory,
    Fifo,
    Socket,
    Device,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PublicationObjectObservation {
    name: String,
    kind: PublicationObjectKind,
    byte_length: u64,
    mode: u32,
    hard_links: u64,
    content_sha256: Option<String>,
    object_generation: u64,
    data_synced: bool,
}

impl PublicationObjectObservation {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::distribution::host_effect) fn new(
        name: String,
        kind: PublicationObjectKind,
        byte_length: u64,
        mode: u32,
        hard_links: u64,
        content_sha256: Option<String>,
        object_generation: u64,
        data_synced: bool,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if !valid_object_name(&name)
            || object_generation == 0
            || content_sha256
                .as_deref()
                .is_some_and(|value| !is_digest(value))
            || (kind == PublicationObjectKind::Missing
                && (byte_length != 0
                    || mode != 0
                    || hard_links != 0
                    || content_sha256.is_some()
                    || data_synced))
            || (kind == PublicationObjectKind::Regular
                && (hard_links != 1 || mode & 0o170000 != 0o100000))
            || (kind != PublicationObjectKind::Regular && data_synced)
            || (kind != PublicationObjectKind::Regular && content_sha256.is_some())
        {
            return Err(recovery_unsafe());
        }
        Ok(Self {
            name,
            kind,
            byte_length,
            mode,
            hard_links,
            content_sha256,
            object_generation,
            data_synced,
        })
    }

    pub(in crate::distribution::host_effect) fn missing(
        name: String,
        object_generation: u64,
    ) -> Result<Self, SupportedHostLifecycleError> {
        Self::new(
            name,
            PublicationObjectKind::Missing,
            0,
            0,
            0,
            None,
            object_generation,
            false,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(in crate::distribution::host_effect) struct ExpectedPublicationObjectIdentity {
    name: String,
    kind: PublicationObjectKind,
    byte_length: u64,
    mode: u32,
    hard_links: u64,
    content_sha256: Option<String>,
    data_synced: bool,
}

impl ExpectedPublicationObjectIdentity {
    pub(in crate::distribution::host_effect) fn missing(
        name: String,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if !valid_object_name(&name) {
            return Err(recovery_unsafe());
        }
        Ok(Self {
            name,
            kind: PublicationObjectKind::Missing,
            byte_length: 0,
            mode: 0,
            hard_links: 0,
            content_sha256: None,
            data_synced: false,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::distribution::host_effect) fn regular(
        name: String,
        byte_length: u64,
        mode: u32,
        hard_links: u64,
        content_sha256: String,
        data_synced: bool,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if !valid_object_name(&name)
            || mode & 0o170000 != 0o100000
            || hard_links != 1
            || !is_digest(&content_sha256)
        {
            return Err(recovery_unsafe());
        }
        Ok(Self {
            name,
            kind: PublicationObjectKind::Regular,
            byte_length,
            mode,
            hard_links,
            content_sha256: Some(content_sha256),
            data_synced,
        })
    }

    fn matches(&self, observed: &PublicationObjectObservation) -> bool {
        self.name == observed.name
            && self.kind == observed.kind
            && self.byte_length == observed.byte_length
            && self.mode == observed.mode
            && self.hard_links == observed.hard_links
            && self.content_sha256 == observed.content_sha256
            && self.data_synced == observed.data_synced
    }

    fn same_renamed_object(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.byte_length == other.byte_length
            && self.mode == other.mode
            && self.hard_links == other.hard_links
            && self.content_sha256 == other.content_sha256
            && self.data_synced == other.data_synced
    }

    fn safe_temporary_metadata_matches(&self, observed: &PublicationObjectObservation) -> bool {
        self.name == observed.name
            && self.kind == observed.kind
            && self.mode == observed.mode
            && self.hard_links == observed.hard_links
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(in crate::distribution::host_effect) struct PublicationExpectation {
    effect_identity_sha256: String,
    prior: ExpectedPublicationObjectIdentity,
    next: ExpectedPublicationObjectIdentity,
    temporary: ExpectedPublicationObjectIdentity,
    publication_identity_sha256: String,
}

impl PublicationExpectation {
    pub(in crate::distribution::host_effect) fn new(
        effect_identity_sha256: String,
        prior: ExpectedPublicationObjectIdentity,
        next: ExpectedPublicationObjectIdentity,
        temporary: ExpectedPublicationObjectIdentity,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if !is_digest(&effect_identity_sha256)
            || !matches!(
                prior.kind,
                PublicationObjectKind::Missing | PublicationObjectKind::Regular
            )
            || next.kind != PublicationObjectKind::Regular
            || temporary.kind != PublicationObjectKind::Regular
            || prior.name != next.name
            || !temporary_name(&temporary.name, &next.name)
            || !next.same_renamed_object(&temporary)
            || (prior.kind == PublicationObjectKind::Regular
                && (!prior.data_synced || prior.mode & 0o222 != 0))
            || !next.data_synced
            || !temporary.data_synced
            || next.mode & 0o222 != 0
            || temporary.mode & 0o222 != 0
        {
            return Err(recovery_unsafe());
        }
        #[derive(Serialize)]
        struct Identity<'a> {
            schema: &'static str,
            effect_identity_sha256: &'a str,
            prior: &'a ExpectedPublicationObjectIdentity,
            next: &'a ExpectedPublicationObjectIdentity,
            temporary: &'a ExpectedPublicationObjectIdentity,
        }
        let publication_identity_sha256 = digest_json(&Identity {
            schema: "harness-ultragoal.expected-publication-identity.v1",
            effect_identity_sha256: &effect_identity_sha256,
            prior: &prior,
            next: &next,
            temporary: &temporary,
        })?;
        Ok(Self {
            effect_identity_sha256,
            prior,
            next,
            temporary,
            publication_identity_sha256,
        })
    }

    pub(in crate::distribution::host_effect) fn publication_identity_sha256(&self) -> &str {
        &self.publication_identity_sha256
    }
}

/// Canonical acknowledgement evidence, not recovery authorization.
///
/// Root recovery authority is issued separately and remains bound to the
/// current classification, coordinator, ledger head, and trusted time.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(in crate::distribution::host_effect) struct PublicationAcknowledgementIdentity {
    schema_version: String,
    effect_identity_sha256: String,
    publication_identity_sha256: String,
    ledger_head: HostEffectLedgerHead,
    acknowledgement_sha256: String,
}

impl PublicationAcknowledgementIdentity {
    pub(in crate::distribution::host_effect) fn new(
        expectation: &PublicationExpectation,
        ledger_head: &HostEffectLedgerHead,
    ) -> Result<Self, SupportedHostLifecycleError> {
        let acknowledgement_sha256 = acknowledgement_digest(
            &expectation.effect_identity_sha256,
            &expectation.publication_identity_sha256,
            ledger_head,
        )?;
        Ok(Self {
            schema_version: "PublicationAcknowledgementIdentity-v1".to_owned(),
            effect_identity_sha256: expectation.effect_identity_sha256.clone(),
            publication_identity_sha256: expectation.publication_identity_sha256.clone(),
            ledger_head: ledger_head.clone(),
            acknowledgement_sha256,
        })
    }

    pub(in crate::distribution::host_effect) fn from_canonical_json(
        bytes: &[u8],
    ) -> Result<Self, SupportedHostLifecycleError> {
        #[derive(Deserialize, Serialize)]
        #[serde(deny_unknown_fields)]
        struct LedgerHeadWire {
            generation: u64,
            head_sha256: String,
        }

        #[derive(Deserialize, Serialize)]
        #[serde(deny_unknown_fields)]
        struct AcknowledgementWire {
            schema_version: String,
            effect_identity_sha256: String,
            publication_identity_sha256: String,
            ledger_head: LedgerHeadWire,
            acknowledgement_sha256: String,
        }

        let wire: AcknowledgementWire =
            serde_json::from_slice(bytes).map_err(|_| recovery_unsafe())?;
        if serde_json::to_vec(&wire).map_err(|_| recovery_unsafe())? != bytes {
            return Err(recovery_unsafe());
        }
        if wire.schema_version != "PublicationAcknowledgementIdentity-v1"
            || !is_digest(&wire.effect_identity_sha256)
            || !is_digest(&wire.publication_identity_sha256)
            || !is_digest(&wire.acknowledgement_sha256)
        {
            return Err(recovery_unsafe());
        }
        let ledger_head =
            HostEffectLedgerHead::new(wire.ledger_head.generation, wire.ledger_head.head_sha256)
                .map_err(|_| recovery_unsafe())?;
        let expected = acknowledgement_digest(
            &wire.effect_identity_sha256,
            &wire.publication_identity_sha256,
            &ledger_head,
        )?;
        if expected != wire.acknowledgement_sha256 {
            return Err(recovery_unsafe());
        }
        Ok(Self {
            schema_version: wire.schema_version,
            effect_identity_sha256: wire.effect_identity_sha256,
            publication_identity_sha256: wire.publication_identity_sha256,
            ledger_head,
            acknowledgement_sha256: wire.acknowledgement_sha256,
        })
    }

    fn matches(
        &self,
        expectation: &PublicationExpectation,
        current_ledger_head: &HostEffectLedgerHead,
    ) -> Result<bool, SupportedHostLifecycleError> {
        Ok(
            self.schema_version == "PublicationAcknowledgementIdentity-v1"
                && self.effect_identity_sha256 == expectation.effect_identity_sha256
                && self.publication_identity_sha256 == expectation.publication_identity_sha256
                && &self.ledger_head == current_ledger_head
                && self.acknowledgement_sha256
                    == acknowledgement_digest(
                        &self.effect_identity_sha256,
                        &self.publication_identity_sha256,
                        &self.ledger_head,
                    )?,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PublicationInventoryObservation {
    scan_generation_before: u64,
    scan_generation_after: u64,
    target: PublicationObjectObservation,
    temporary_objects: Vec<PublicationObjectObservation>,
    expectation: PublicationExpectation,
    current_ledger_head: HostEffectLedgerHead,
    acknowledgement: Option<PublicationAcknowledgementIdentity>,
}

impl PublicationInventoryObservation {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::distribution::host_effect) fn new(
        scan_generation_before: u64,
        scan_generation_after: u64,
        target: PublicationObjectObservation,
        temporary_objects: Vec<PublicationObjectObservation>,
        expectation: PublicationExpectation,
        current_ledger_head: HostEffectLedgerHead,
        acknowledgement: Option<PublicationAcknowledgementIdentity>,
    ) -> Result<Self, SupportedHostLifecycleError> {
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

pub(super) fn propose_recovery(
    classification: &PublicationClassification,
    authorization: RecoveryAuthorization,
    coordinator_binding_sha256: &str,
    head: &HostEffectLedgerHead,
    trusted_now_unix_ms: u64,
) -> Result<RecoveryProposal, SupportedHostLifecycleError> {
    let expected = authorization_digest(
        &authorization.schema_version,
        &authorization.classification_sha256,
        &authorization.coordinator_binding_sha256,
        &authorization.ledger_head,
        authorization.issued_at_unix_ms,
        authorization.expires_at_unix_ms,
        &authorization.nonce_sha256,
    )?;
    if expected != authorization.authorization_sha256
        || authorization.schema_version != RECOVERY_AUTHORIZATION_SCHEMA
        || authorization.classification_sha256 != classification.evidence_sha256
        || authorization.coordinator_binding_sha256 != coordinator_binding_sha256
        || &authorization.ledger_head != head
        || trusted_now_unix_ms < authorization.issued_at_unix_ms
        || trusted_now_unix_ms > authorization.expires_at_unix_ms
    {
        return Err(recovery_authorization_required());
    }
    let action = match classification.id {
        PublicationClassificationId::CleanPriorState => {
            RecoveryProposalAction::ResumeWithoutMutation
        }
        PublicationClassificationId::InterruptedBeforeTempWrite
        | PublicationClassificationId::InterruptedDuringTempFsync
        | PublicationClassificationId::InterruptedBeforeRename
        | PublicationClassificationId::OrphanedTemporaryObject => {
            RecoveryProposalAction::QuarantineTemporaryObjectForReview
        }
        PublicationClassificationId::CommittedBeforeAcknowledgement => {
            RecoveryProposalAction::ReconcileCommittedStateBeforeAcknowledgement
        }
        _ => {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::RecoveryUnsafe,
            ));
        }
    };
    #[derive(Serialize)]
    struct Proposal<'a> {
        schema: &'static str,
        action: RecoveryProposalAction,
        classification_sha256: &'a str,
        authorization_sha256: &'a str,
        authorized_ledger_head: &'a HostEffectLedgerHead,
        automatic_cleanup: bool,
    }
    let proposal_sha256 = digest_json(&Proposal {
        schema: "harness-ultragoal.host-effect-recovery-proposal.v2",
        action,
        classification_sha256: &classification.evidence_sha256,
        authorization_sha256: &authorization.authorization_sha256,
        authorized_ledger_head: &authorization.ledger_head,
        automatic_cleanup: false,
    })?;
    Ok(RecoveryProposal {
        action,
        classification_sha256: classification.evidence_sha256.clone(),
        proposal_sha256,
        automatic_cleanup: false,
    })
}

fn classify_publication(
    observation: &PublicationInventoryObservation,
) -> Result<PublicationClassification, SupportedHostLifecycleError> {
    let id = if observation.scan_generation_before != observation.scan_generation_after
        || observation.target.object_generation != observation.scan_generation_before
        || observation
            .temporary_objects
            .iter()
            .any(|object| object.object_generation != observation.scan_generation_before)
    {
        PublicationClassificationId::ObservationRace
    } else if !matches!(
        observation.target.kind,
        PublicationObjectKind::Missing | PublicationObjectKind::Regular
    ) || observation.temporary_objects.iter().any(|object| {
        object.kind != PublicationObjectKind::Regular
            || object.mode & 0o170000 != 0o100000
            || object.hard_links != 1
    }) {
        PublicationClassificationId::UnsafeSpecialObject
    } else if observation.temporary_objects.len() > 1 {
        PublicationClassificationId::MultipleTemporaryObjects
    } else if let Some(acknowledgement) = &observation.acknowledgement {
        if exact_next_target(observation)
            && observation.temporary_objects.is_empty()
            && acknowledgement
                .matches(&observation.expectation, &observation.current_ledger_head)?
        {
            PublicationClassificationId::AcknowledgedCommitted
        } else {
            PublicationClassificationId::FalsePassReceipt
        }
    } else if let Some(temporary) = observation.temporary_objects.first() {
        if exact_next_target(observation) {
            PublicationClassificationId::OrphanedTemporaryObject
        } else if !prior_target_matches(observation)
            || !observation
                .expectation
                .temporary
                .safe_temporary_metadata_matches(temporary)
        {
            PublicationClassificationId::OrphanedTemporaryObject
        } else if temporary.byte_length == 0
            && temporary.content_sha256.is_none()
            && !temporary.data_synced
        {
            PublicationClassificationId::InterruptedBeforeTempWrite
        } else if !temporary.data_synced {
            PublicationClassificationId::InterruptedDuringTempFsync
        } else if observation.expectation.temporary.matches(temporary) {
            PublicationClassificationId::InterruptedBeforeRename
        } else {
            PublicationClassificationId::OrphanedTemporaryObject
        }
    } else if exact_next_target(observation) {
        PublicationClassificationId::CommittedBeforeAcknowledgement
    } else if prior_target_matches(observation) {
        PublicationClassificationId::CleanPriorState
    } else {
        PublicationClassificationId::UnknownState
    };
    #[derive(Serialize)]
    struct Classification<'a> {
        schema: &'static str,
        id: PublicationClassificationId,
        observation: &'a PublicationInventoryObservation,
    }
    Ok(PublicationClassification {
        id,
        evidence_sha256: digest_json(&Classification {
            schema: "harness-ultragoal.prepublication-recovery-classification.v2",
            id,
            observation,
        })?,
        temporary_object: observation
            .temporary_objects
            .first()
            .map(|object| object.name.clone()),
    })
}

fn exact_next_target(observation: &PublicationInventoryObservation) -> bool {
    observation.expectation.next.matches(&observation.target)
}

fn prior_target_matches(observation: &PublicationInventoryObservation) -> bool {
    observation.expectation.prior.matches(&observation.target)
}

fn temporary_name(value: &str, target: &str) -> bool {
    let Some(rest) = value.strip_prefix(&format!(".{target}.")) else {
        return false;
    };
    let Some(hex) = rest.strip_suffix(".tmp") else {
        return false;
    };
    hex.len() == 16
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_object_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !matches!(value, "." | "..")
        && !value
            .bytes()
            .any(|byte| byte == b'/' || byte == 0 || byte.is_ascii_control())
}

fn is_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn authorization_digest(
    schema_version: &str,
    classification_sha256: &str,
    coordinator_binding_sha256: &str,
    ledger_head: &HostEffectLedgerHead,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    nonce_sha256: &str,
) -> Result<String, SupportedHostLifecycleError> {
    if schema_version != RECOVERY_AUTHORIZATION_SCHEMA
        || !is_digest(classification_sha256)
        || !is_digest(coordinator_binding_sha256)
        || !is_digest(ledger_head.head_sha256())
        || issued_at_unix_ms == 0
        || issued_at_unix_ms.checked_add(60_000) != Some(expires_at_unix_ms)
        || !is_digest(nonce_sha256)
    {
        return Err(recovery_authorization_required());
    }
    #[derive(Serialize)]
    struct Authorization<'a> {
        schema: &'a str,
        classification_sha256: &'a str,
        coordinator_binding_sha256: &'a str,
        ledger_head: &'a HostEffectLedgerHead,
        issued_at_unix_ms: u64,
        expires_at_unix_ms: u64,
        nonce_sha256: &'a str,
    }
    digest_json(&Authorization {
        schema: schema_version,
        classification_sha256,
        coordinator_binding_sha256,
        ledger_head,
        issued_at_unix_ms,
        expires_at_unix_ms,
        nonce_sha256,
    })
}

fn acknowledgement_digest(
    effect_identity_sha256: &str,
    publication_identity_sha256: &str,
    ledger_head: &HostEffectLedgerHead,
) -> Result<String, SupportedHostLifecycleError> {
    if !is_digest(effect_identity_sha256) || !is_digest(publication_identity_sha256) {
        return Err(recovery_unsafe());
    }
    #[derive(Serialize)]
    struct Acknowledgement<'a> {
        schema: &'static str,
        effect_identity_sha256: &'a str,
        publication_identity_sha256: &'a str,
        ledger_head: &'a HostEffectLedgerHead,
    }
    digest_json(&Acknowledgement {
        schema: "harness-ultragoal.publication-acknowledgement-identity.v1",
        effect_identity_sha256,
        publication_identity_sha256,
        ledger_head,
    })
}

fn digest_json(value: &impl Serialize) -> Result<String, SupportedHostLifecycleError> {
    serde_json::to_vec(value)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::RecoveryUnsafe))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn recovery_authorization_required() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired)
}

fn recovery_unsafe() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::RecoveryUnsafe)
}

#[cfg(test)]
mod authorization_digest_tests {
    use super::*;

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    #[test]
    fn malformed_or_unknown_canonical_authorization_controls_fail_closed() {
        let head = HostEffectLedgerHead::new(0, digest('c')).unwrap();
        assert!(
            authorization_digest(
                RECOVERY_AUTHORIZATION_SCHEMA,
                &digest('a'),
                &digest('b'),
                &head,
                100_000,
                160_000,
                &digest('d'),
            )
            .is_ok()
        );

        for schema_version in [
            "",
            "harness-ultragoal.host-effect-recovery-authorization.v1",
            "harness-ultragoal.host-effect-recovery-authorization.v3",
            "harness-ultragoal.host-effect-recovery-authorization.v2 ",
        ] {
            assert_eq!(
                authorization_digest(
                    schema_version,
                    &digest('a'),
                    &digest('b'),
                    &head,
                    100_000,
                    160_000,
                    &digest('d'),
                )
                .unwrap_err()
                .id(),
                SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
            );
        }

        for (classification, coordinator, nonce) in [
            ("not-a-digest".to_owned(), digest('b'), digest('d')),
            (digest('a'), "sha256:ABC".to_owned(), digest('d')),
            (digest('a'), digest('b'), String::new()),
        ] {
            assert_eq!(
                authorization_digest(
                    RECOVERY_AUTHORIZATION_SCHEMA,
                    &classification,
                    &coordinator,
                    &head,
                    100_000,
                    160_000,
                    &nonce,
                )
                .unwrap_err()
                .id(),
                SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
            );
        }
        assert_eq!(
            authorization_digest(
                RECOVERY_AUTHORIZATION_SCHEMA,
                &digest('a'),
                &digest('b'),
                &head,
                100_000,
                159_999,
                &digest('d'),
            )
            .unwrap_err()
            .id(),
            SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
        );
    }
}
