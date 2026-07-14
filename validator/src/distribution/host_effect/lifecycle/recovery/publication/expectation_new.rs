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

pub(in crate::distribution::host_effect) struct PublicationInventoryObservationRequest {
    pub scan_generation_before: u64,
    pub scan_generation_after: u64,
    pub target: PublicationObjectObservation,
    pub temporary_objects: Vec<PublicationObjectObservation>,
    pub expectation: PublicationExpectation,
    pub current_ledger_head: HostEffectLedgerHead,
    pub acknowledgement: Option<PublicationAcknowledgementIdentity>,
}
