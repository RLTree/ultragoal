//! Internal supported-host transaction executor candidate.
//!
//! The accepted lifecycle coordinator remains the only source of an
//! `AuthorizedHostEffect`. Darwin remains unsupported at that coordinator
//! boundary because the host has no retained-descriptor execution primitive.
//! This module does not add a public route or weaken that decision. It supplies
//! the dependency-closed transaction consumer, a descriptor-capable process
//! backend for Linux/FreeBSD, and descriptor-relative durable publication used
//! by the executor after a retained-authority handoff.

mod model;
mod process;
mod target;

pub(crate) use model::{
    HostEffectCancellation, HostEffectExecutionPolicy, HostEffectExecutionReceipt,
    HostEffectExecutorErrorId, HostEffectExecutorFailure,
    HostEffectPostPublicationRecoveryClassification, HostEffectPostReservationLedgerClassification,
    HostEffectPostReservationPublicationClassification, HostEffectRecoveryHandoff,
    HostEffectTerminalRecoveryClassification,
};
pub(crate) use process::{
    NativeRetainedDescriptorProcessBackend, RetainedDescriptorProcessBackend,
};
pub(crate) use target::{ConfinedHostEffectTarget, ConfinedHostEffectTargetObserver};

use self::model::{CommandCaptureDigest, digest_json};
use self::process::BackendFailure;
use self::target::{CommittedPublication, PublicationFailure};
use super::lifecycle::{
    DescriptorExecutionCapability, DescriptorExecutionHandoff, DescriptorExecutionPlatform,
    HostTargetLease, PublicationAcknowledgementIdentity, PublicationClassificationId,
    PublicationInventoryObservation, RootTrustedClock,
};
use super::{
    AuthorizedHostEffect, DurableHostEffectLedger, HostEffectLedgerHead, HostEffectLedgerRecord,
    HostEffectOutcome, HostEffectState, HostEffectTransition,
};
use serde::Serialize;

#[derive(Clone, Copy)]
struct PriorPublicationEvidence<'a> {
    publication_identity_sha256: Option<&'a str>,
    observation: Option<&'a PublicationInventoryObservation>,
}

struct PostReservationLedgerEvidence {
    ledger_head: HostEffectLedgerHead,
    ledger_record: HostEffectLedgerRecord,
    exact_current_observation: bool,
    classification: HostEffectPostReservationLedgerClassification,
    terminal_state: Option<HostEffectState>,
    observation_error_id: Option<HostEffectExecutorErrorId>,
}

pub(crate) struct SupportedHostEffectExecutor<'a> {
    ledger: &'a dyn DurableHostEffectLedger,
    target: ConfinedHostEffectTarget,
    backend: &'a mut dyn RetainedDescriptorProcessBackend,
    policy: HostEffectExecutionPolicy,
}

impl std::fmt::Debug for SupportedHostEffectExecutor<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SupportedHostEffectExecutor")
            .field("target", &self.target)
            .field("policy_sha256", &self.policy.policy_sha256())
            .finish_non_exhaustive()
    }
}

impl<'a> SupportedHostEffectExecutor<'a> {
    pub(in crate::distribution::host_effect) fn new(
        ledger: &'a dyn DurableHostEffectLedger,
        target: ConfinedHostEffectTarget,
        backend: &'a mut dyn RetainedDescriptorProcessBackend,
        policy: HostEffectExecutionPolicy,
    ) -> Self {
        Self {
            ledger,
            target,
            backend,
            policy,
        }
    }

    /// Consumes the accepted opaque handoff while it retains the exact target
    /// lease and authorized effect. No `AuthorizedHostEffect` escapes this
    /// synchronous call.
    pub(in crate::distribution::host_effect) fn execute_handoff(
        &mut self,
        handoff: DescriptorExecutionHandoff,
        clock: &mut dyn RootTrustedClock,
        cancellation: &HostEffectCancellation,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        handoff.with_retained_authority(|capability, effect, lease| {
            if capability.platform() != DescriptorExecutionPlatform::current() {
                let effect_identity_sha256 = self
                    .effect_identity(capability, effect)
                    .unwrap_or_else(|_| fallback_effect_identity(effect));
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    &effect_identity_sha256,
                    HostEffectExecutorErrorId::UnsupportedPlatform,
                    vec![HostEffectExecutorErrorId::UnsupportedPlatform],
                    None,
                ));
            }
            self.execute_authorized(capability, effect, lease, clock, cancellation)
        })
    }

    fn execute_authorized(
        &mut self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
        target_lease: &mut dyn HostTargetLease,
        clock: &mut dyn RootTrustedClock,
        cancellation: &HostEffectCancellation,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        let boundary_effect_identity = self
            .effect_identity(capability, effect)
            .unwrap_or_else(|_| fallback_effect_identity(effect));
        match self.execute_reserved(capability, effect, target_lease, clock, cancellation) {
            Err(failure) if failure.recovery().is_none() => Err(self
                .post_reservation_recovery_failure(
                    effect,
                    &boundary_effect_identity,
                    failure.id(),
                    vec![failure.id()],
                    None,
                )),
            result => result,
        }
    }

    /// Private fallible implementation behind `execute_authorized`'s
    /// post-reservation identity guard. No caller may expose this result
    /// directly after the opaque handoff has been consumed.
    fn execute_reserved(
        &mut self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
        target_lease: &mut dyn HostTargetLease,
        clock: &mut dyn RootTrustedClock,
        cancellation: &HostEffectCancellation,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        if let Err(failure) = self.preflight(effect, target_lease) {
            if self.has_current_in_flight_record(effect) {
                return self.finish_backend_failure(
                    effect,
                    &fallback_effect_identity(effect),
                    Vec::new(),
                    BackendFailure::before_start(failure.id()),
                    clock,
                );
            }
            if failure.id() == HostEffectExecutorErrorId::LedgerSubstitution {
                let effect_identity_sha256 = self
                    .effect_identity(capability, effect)
                    .unwrap_or_else(|_| fallback_effect_identity(effect));
                return Err(self.identity_bearing_preflight_ledger_failure(
                    effect,
                    &effect_identity_sha256,
                    failure.id(),
                ));
            }
            return Err(failure);
        }
        let effect_identity_sha256 = match self.effect_identity(capability, effect) {
            Ok(identity) => identity,
            Err(failure) => {
                return self.finish_backend_failure(
                    effect,
                    &fallback_effect_identity(effect),
                    Vec::new(),
                    BackendFailure::before_start(failure.id()),
                    clock,
                );
            }
        };
        let receipt_name = receipt_name(effect.permit().permit_id())?;
        // Collision inspection is deliberately before the first backend call.
        // Preparing the final bytes happens later, but any prior receipt or
        // temporary namespace is already a replay/recovery boundary.
        if let Err(failure) = self.target.require_clean_publication_name(&receipt_name) {
            return self.finish_backend_failure(
                effect,
                &effect_identity_sha256,
                Vec::new(),
                BackendFailure::before_start(failure.id()),
                clock,
            );
        }

        let mut command_digests = Vec::with_capacity(effect.plan().commands().len());
        for (command_index, command) in effect.plan().commands().iter().enumerate() {
            if cancellation.is_cancelled() {
                return self.finish_backend_failure(
                    effect,
                    &effect_identity_sha256,
                    command_digests,
                    BackendFailure::before_start(HostEffectExecutorErrorId::Cancelled),
                    clock,
                );
            }
            if effect.executable().revalidate().is_err() {
                return self.finish_backend_failure(
                    effect,
                    &effect_identity_sha256,
                    command_digests,
                    BackendFailure {
                        id: HostEffectExecutorErrorId::ExecutableMutation,
                        started: command_index != 0,
                        capture: model::CommandCapture::empty_failure(),
                    },
                    clock,
                );
            }
            let capture = match self.backend.execute(
                capability,
                effect.executable(),
                command,
                &self.policy,
                cancellation,
            ) {
                Ok(capture) => capture,
                Err(failure) => {
                    return self.finish_backend_failure(
                        effect,
                        &effect_identity_sha256,
                        command_digests,
                        failure,
                        clock,
                    );
                }
            };
            let digest = match capture.digest(command_index) {
                Ok(digest) => digest,
                Err(failure) => {
                    return self.finish_started_failure(
                        effect,
                        &effect_identity_sha256,
                        command_digests,
                        failure.id(),
                        clock,
                    );
                }
            };
            if capture.exit_code != 0 {
                return self.finish_backend_failure(
                    effect,
                    &effect_identity_sha256,
                    command_digests,
                    BackendFailure {
                        id: HostEffectExecutorErrorId::ProcessFailed,
                        started: true,
                        capture,
                    },
                    clock,
                );
            }
            command_digests.push(digest);
            if effect.executable().revalidate().is_err() {
                return self.finish_started_failure(
                    effect,
                    &effect_identity_sha256,
                    command_digests,
                    HostEffectExecutorErrorId::ExecutableMutation,
                    clock,
                );
            }
        }

        let completed_at_unix_ms = match clock.sample() {
            Ok(sample) => sample.unix_ms(),
            Err(_clock_failure) => {
                return self.finish_terminal_with_timestamp(
                    effect,
                    &effect_identity_sha256,
                    command_digests,
                    HostEffectExecutorErrorId::Io,
                    HostEffectState::Ambiguous,
                    0,
                );
            }
        };
        let command_output_sha256 = command_digests
            .iter()
            .map(|capture| capture.combined_sha256.clone())
            .collect::<Vec<_>>();
        let outcome_sha256 = outcome_digest(
            &effect_identity_sha256,
            effect,
            HostEffectState::Settled,
            &command_digests,
            self.policy.policy_sha256(),
            completed_at_unix_ms,
        )?;
        let publication_bytes = publication_bytes(
            &effect_identity_sha256,
            effect,
            &command_digests,
            self.policy.policy_sha256(),
            &outcome_sha256,
            completed_at_unix_ms,
        )?;
        let prepared =
            match self
                .target
                .prepare(&effect_identity_sha256, &receipt_name, publication_bytes)
            {
                Ok(prepared) => prepared,
                Err(failure) => {
                    return self.finish_terminal_with_timestamp(
                        effect,
                        &effect_identity_sha256,
                        command_digests,
                        failure.id(),
                        HostEffectState::Ambiguous,
                        completed_at_unix_ms,
                    );
                }
            };
        let committed = match self
            .target
            .publish(prepared, effect.record().current_head())
        {
            Ok(committed) => committed,
            Err(failure) => {
                return self.finish_publication_failure(
                    effect,
                    &effect_identity_sha256,
                    command_digests,
                    outcome_sha256,
                    completed_at_unix_ms,
                    failure,
                );
            }
        };

        // The publication object is durable before the terminal ledger state.
        // A crash here is classifiable as committed-before-acknowledgement.
        let terminal = match self.transition_terminal(
            effect,
            HostEffectState::Settled,
            outcome_sha256.clone(),
        ) {
            Ok(terminal) => terminal,
            Err(failure) => {
                return Err(self.committed_before_terminal_recovery_failure(
                    effect,
                    &effect_identity_sha256,
                    &committed,
                    vec![failure.id()],
                ));
            }
        };
        let acknowledgement = PublicationAcknowledgementIdentity::new(
            &committed.expectation,
            terminal.current_head(),
        )
        .map_err(|_| {
            self.committed_recovery_failure(
                effect,
                &effect_identity_sha256,
                &committed,
                terminal.current_head().clone(),
                HostEffectExecutorErrorId::PartialAcknowledgement,
            )
        })?;
        let acknowledgement_json = serde_json::to_vec(&acknowledgement).map_err(|_| {
            self.committed_recovery_failure(
                effect,
                &effect_identity_sha256,
                &committed,
                terminal.current_head().clone(),
                HostEffectExecutorErrorId::PartialAcknowledgement,
            )
        })?;
        let decoded =
            PublicationAcknowledgementIdentity::from_canonical_json(&acknowledgement_json)
                .map_err(|_| {
                    self.committed_recovery_failure(
                        effect,
                        &effect_identity_sha256,
                        &committed,
                        terminal.current_head().clone(),
                        HostEffectExecutorErrorId::PartialAcknowledgement,
                    )
                })?;
        if decoded != acknowledgement {
            return Err(self.committed_recovery_failure(
                effect,
                &effect_identity_sha256,
                &committed,
                terminal.current_head().clone(),
                HostEffectExecutorErrorId::FalsePassReceipt,
            ));
        }
        let (_, classification) = self
            .target
            .acknowledge(
                &committed,
                acknowledgement.clone(),
                terminal.current_head().clone(),
            )
            .map_err(|_| {
                self.committed_recovery_failure(
                    effect,
                    &effect_identity_sha256,
                    &committed,
                    terminal.current_head().clone(),
                    HostEffectExecutorErrorId::FalsePassReceipt,
                )
            })?;
        if classification.id() != PublicationClassificationId::AcknowledgedCommitted {
            return Err(self.committed_recovery_failure(
                effect,
                &effect_identity_sha256,
                &committed,
                terminal.current_head().clone(),
                HostEffectExecutorErrorId::FalsePassReceipt,
            ));
        }
        let outcome = HostEffectOutcome::new(
            effect.permit().permit_id().to_owned(),
            HostEffectState::Settled,
            command_output_sha256.clone(),
            Some(
                committed
                    .expectation
                    .publication_identity_sha256()
                    .to_owned(),
            ),
            outcome_sha256,
            completed_at_unix_ms,
        )
        .map_err(|_| {
            self.committed_recovery_failure(
                effect,
                &effect_identity_sha256,
                &committed,
                terminal.current_head().clone(),
                HostEffectExecutorErrorId::FalsePassReceipt,
            )
        })?;
        Ok(HostEffectExecutionReceipt::new(
            effect_identity_sha256,
            outcome,
            terminal.current_head().clone(),
            command_output_sha256,
            acknowledgement,
            acknowledgement_json,
            classification,
        ))
    }

    fn preflight(
        &self,
        effect: &AuthorizedHostEffect,
        target_lease: &mut dyn HostTargetLease,
    ) -> Result<(), HostEffectExecutorFailure> {
        let current = self
            .ledger
            .read(effect.permit().permit_id())
            .map_err(|_| ledger_failure())?;
        match current {
            Some(record) if &record == effect.record() => {}
            Some(record)
                if matches!(
                    record.state(),
                    HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
                ) =>
            {
                return Err(HostEffectExecutorFailure::new(
                    HostEffectExecutorErrorId::Replay,
                ));
            }
            _ => return Err(ledger_failure()),
        }
        if effect.record().state() != HostEffectState::InFlight
            || self.ledger.head().map_err(|_| ledger_failure())? != *effect.record().current_head()
        {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::LedgerRollback,
            ));
        }
        self.target.revalidate_anchor()?;
        let lease_before = target_lease.identity().clone();
        if &lease_before != self.target.expected_target() {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::TargetSubstitution,
            ));
        }
        let lease_after = target_lease.revalidate().map_err(|_| target_failure())?;
        if lease_before != lease_after || &lease_after != self.target.expected_target() {
            return Err(target_failure());
        }
        effect.executable().revalidate().map_err(|_| {
            HostEffectExecutorFailure::new(HostEffectExecutorErrorId::ExecutableMutation)
        })?;
        Ok(())
    }

    fn effect_identity(
        &self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
    ) -> Result<String, HostEffectExecutorFailure> {
        #[derive(Serialize)]
        struct Identity<'a> {
            schema: &'static str,
            permit_id: &'a str,
            permit_binding_sha256: &'a str,
            semantic_key_sha256: &'a str,
            command_plan_sha256: &'a str,
            executable_identity_sha256: String,
            target_identity_sha256: &'a str,
            in_flight_ledger_head: &'a HostEffectLedgerHead,
            descriptor_capability_sha256: &'a str,
            environment_sha256: &'a str,
            execution_policy_sha256: &'a str,
        }
        digest_json(&Identity {
            schema: "harness-ultragoal.authorized-host-effect-execution.v1",
            permit_id: effect.permit().permit_id(),
            permit_binding_sha256: effect.permit().binding_sha256(),
            semantic_key_sha256: effect.permit().semantic_key_sha256(),
            command_plan_sha256: effect.plan().plan_sha256(),
            executable_identity_sha256: effect.executable().identity().binding_sha256().map_err(
                |_| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::ExecutableMutation),
            )?,
            target_identity_sha256: self.target.expected_target().target_sha256(),
            in_flight_ledger_head: effect.record().current_head(),
            descriptor_capability_sha256: capability.capability_sha256(),
            environment_sha256: self.policy.environment_sha256(),
            execution_policy_sha256: self.policy.policy_sha256(),
        })
    }

    fn has_current_in_flight_record(&self, effect: &AuthorizedHostEffect) -> bool {
        effect.record().state() == HostEffectState::InFlight
            && self
                .ledger
                .read(effect.permit().permit_id())
                .ok()
                .flatten()
                .is_some_and(|current| current == *effect.record())
            && self
                .ledger
                .head()
                .is_ok_and(|head| head == *effect.record().current_head())
    }

    fn transition_terminal(
        &self,
        effect: &AuthorizedHostEffect,
        state: HostEffectState,
        outcome_sha256: String,
    ) -> Result<HostEffectLedgerRecord, HostEffectExecutorFailure> {
        let before = self
            .ledger
            .read(effect.permit().permit_id())
            .map_err(|_| ledger_failure())?
            .ok_or_else(ledger_failure)?;
        if before.state() != HostEffectState::InFlight
            || before.reservation() != effect.record().reservation()
            || self.ledger.head().map_err(|_| ledger_failure())? != *before.current_head()
        {
            return Err(ledger_failure());
        }
        let terminal = self
            .ledger
            .transition(
                HostEffectTransition::new(
                    effect.permit().permit_id().to_owned(),
                    HostEffectState::InFlight,
                    state,
                    before.current_head().clone(),
                    Some(outcome_sha256),
                )
                .map_err(|_| ledger_failure())?,
            )
            .map_err(|_| ledger_failure())?;
        if terminal.state() != state
            || terminal.reservation() != effect.record().reservation()
            || self.ledger.head().map_err(|_| ledger_failure())? != *terminal.current_head()
            || self
                .ledger
                .read(effect.permit().permit_id())
                .map_err(|_| ledger_failure())?
                != Some(terminal.clone())
        {
            return Err(ledger_failure());
        }
        Ok(terminal)
    }

    fn finish_backend_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        mut completed: Vec<CommandCaptureDigest>,
        failure: BackendFailure,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        if failure.capture.stdout.len() <= model::EXACT_OUTPUT_LIMIT_BYTES
            && failure.capture.stderr.len() <= model::EXACT_OUTPUT_LIMIT_BYTES
        {
            if let Ok(digest) = failure.capture.digest(completed.len()) {
                completed.push(digest);
            }
        }
        let state = if failure.started {
            HostEffectState::Ambiguous
        } else {
            HostEffectState::Failed
        };
        let mut originating_error_ids = vec![failure.id];
        let completed_at_unix_ms = match clock.sample() {
            Ok(sample) => sample.unix_ms(),
            Err(_clock_failure) => {
                append_error_cause(&mut originating_error_ids, HostEffectExecutorErrorId::Io);
                0
            }
        };
        let returned_error_id = *originating_error_ids
            .last()
            .expect("backend failure always supplies an originating cause");
        let outcome_sha256 = match outcome_digest(
            effect_identity_sha256,
            effect,
            state,
            &completed,
            self.policy.policy_sha256(),
            completed_at_unix_ms,
        ) {
            Ok(outcome_sha256) => outcome_sha256,
            Err(outcome_failure) => {
                append_error_cause(&mut originating_error_ids, outcome_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    None,
                ));
            }
        };
        let outcome = match terminal_outcome(
            effect,
            state,
            &completed,
            outcome_sha256.clone(),
            completed_at_unix_ms,
        ) {
            Ok(outcome) => outcome,
            Err(outcome_failure) => {
                append_error_cause(&mut originating_error_ids, outcome_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    None,
                ));
            }
        };
        match self.transition_terminal(effect, state, outcome_sha256) {
            Ok(terminal) => Err(self.verified_terminal_failure(
                effect,
                effect_identity_sha256,
                terminal,
                outcome,
                returned_error_id,
                originating_error_ids,
            )),
            Err(transition_failure) => {
                append_error_cause(&mut originating_error_ids, transition_failure.id());
                Err(self.prepublication_terminal_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    outcome,
                    originating_error_ids,
                ))
            }
        }
    }

    fn finish_started_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        completed: Vec<CommandCaptureDigest>,
        id: HostEffectExecutorErrorId,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        self.finish_backend_failure(
            effect,
            effect_identity_sha256,
            completed,
            BackendFailure {
                id,
                started: true,
                capture: model::CommandCapture::empty_failure(),
            },
            clock,
        )
    }

    fn finish_terminal_with_timestamp(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        completed: Vec<CommandCaptureDigest>,
        id: HostEffectExecutorErrorId,
        state: HostEffectState,
        completed_at_unix_ms: u64,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        let mut originating_error_ids = vec![id];
        let outcome_sha256 = match outcome_digest(
            effect_identity_sha256,
            effect,
            state,
            &completed,
            self.policy.policy_sha256(),
            completed_at_unix_ms,
        ) {
            Ok(outcome_sha256) => outcome_sha256,
            Err(outcome_failure) => {
                append_error_cause(&mut originating_error_ids, outcome_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    None,
                ));
            }
        };
        let outcome = match terminal_outcome(
            effect,
            state,
            &completed,
            outcome_sha256.clone(),
            completed_at_unix_ms,
        ) {
            Ok(outcome) => outcome,
            Err(outcome_failure) => {
                append_error_cause(&mut originating_error_ids, outcome_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    None,
                ));
            }
        };
        match self.transition_terminal(effect, state, outcome_sha256) {
            Ok(terminal) => Err(self.verified_terminal_failure(
                effect,
                effect_identity_sha256,
                terminal,
                outcome,
                id,
                originating_error_ids,
            )),
            Err(transition_failure) => {
                append_error_cause(&mut originating_error_ids, transition_failure.id());
                Err(self.prepublication_terminal_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    outcome,
                    originating_error_ids,
                ))
            }
        }
    }

    fn verified_terminal_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        terminal: HostEffectLedgerRecord,
        outcome: HostEffectOutcome,
        returned_error_id: HostEffectExecutorErrorId,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
    ) -> HostEffectExecutorFailure {
        let terminal_state = terminal.state();
        let recovery = HostEffectRecoveryHandoff::terminal_transition(
            effect_identity_sha256.to_owned(),
            effect.permit().permit_id().to_owned(),
            terminal.current_head().clone(),
            terminal,
            true,
            outcome,
            originating_error_ids,
            HostEffectTerminalRecoveryClassification::TerminalCommittedAndVerified,
        );
        debug_assert!(recovery.verify_binding());
        HostEffectExecutorFailure::with_recovery(returned_error_id, Some(terminal_state), recovery)
    }

    fn prepublication_terminal_recovery_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        outcome: HostEffectOutcome,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
    ) -> HostEffectExecutorFailure {
        let (ledger_record, ledger_head, exact, classification, terminal_state) = self
            .reobserve_terminal_failure(effect, &outcome)
            .unwrap_or_else(|| {
                (
                    effect.record().clone(),
                    effect.record().current_head().clone(),
                    false,
                    HostEffectTerminalRecoveryClassification::LedgerObservationUnavailable,
                    None,
                )
            });
        let recovery = HostEffectRecoveryHandoff::terminal_transition(
            effect_identity_sha256.to_owned(),
            effect.permit().permit_id().to_owned(),
            ledger_head,
            ledger_record,
            exact,
            outcome,
            originating_error_ids,
            classification,
        );
        debug_assert!(recovery.verify_binding());
        HostEffectExecutorFailure::with_recovery(
            HostEffectExecutorErrorId::RecoveryRequired,
            terminal_state,
            recovery,
        )
    }

    fn identity_bearing_preflight_ledger_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        originating_error_id: HostEffectExecutorErrorId,
    ) -> HostEffectExecutorFailure {
        let outcome_sha256 = outcome_digest(
            effect_identity_sha256,
            effect,
            HostEffectState::Failed,
            &[],
            self.policy.policy_sha256(),
            0,
        )
        .expect("preflight recovery outcome serialization is infallible");
        let outcome = terminal_outcome(effect, HostEffectState::Failed, &[], outcome_sha256, 0)
            .expect("preflight recovery outcome construction is infallible");
        let recovery = HostEffectRecoveryHandoff::terminal_transition(
            effect_identity_sha256.to_owned(),
            effect.permit().permit_id().to_owned(),
            effect.record().current_head().clone(),
            effect.record().clone(),
            false,
            outcome,
            vec![originating_error_id],
            HostEffectTerminalRecoveryClassification::LedgerObservationUnavailable,
        );
        debug_assert!(recovery.verify_binding());
        HostEffectExecutorFailure::with_recovery(
            HostEffectExecutorErrorId::RecoveryRequired,
            None,
            recovery,
        )
    }

    fn reobserve_terminal_failure(
        &self,
        effect: &AuthorizedHostEffect,
        outcome: &HostEffectOutcome,
    ) -> Option<(
        HostEffectLedgerRecord,
        HostEffectLedgerHead,
        bool,
        HostEffectTerminalRecoveryClassification,
        Option<HostEffectState>,
    )> {
        const MAX_STABLE_OBSERVATION_ATTEMPTS: usize = 3;
        for _ in 0..MAX_STABLE_OBSERVATION_ATTEMPTS {
            let Ok(head_before) = self.ledger.head() else {
                continue;
            };
            let Ok(Some(record_before)) = self.ledger.read(effect.permit().permit_id()) else {
                continue;
            };
            let Ok(Some(record_after)) = self.ledger.read(effect.permit().permit_id()) else {
                continue;
            };
            let Ok(head_after) = self.ledger.head() else {
                continue;
            };
            if head_before != head_after || record_before != record_after {
                continue;
            }
            if head_after != *record_after.current_head()
                || record_after.reservation().permit_id() != effect.permit().permit_id()
            {
                continue;
            }
            if record_after == *effect.record() {
                return Some((
                    record_after,
                    head_after,
                    true,
                    HostEffectTerminalRecoveryClassification::StillInFlight,
                    None,
                ));
            }
            if record_after.reservation() == effect.record().reservation()
                && record_after.state() == outcome.state
                && record_after.outcome_sha256.as_deref() == Some(outcome.outcome_sha256.as_str())
            {
                return Some((
                    record_after,
                    head_after,
                    true,
                    HostEffectTerminalRecoveryClassification::TerminalCommittedButUnverifiable,
                    Some(outcome.state),
                ));
            }
            return Some((
                record_after,
                head_after,
                true,
                HostEffectTerminalRecoveryClassification::LedgerObservationRejected,
                None,
            ));
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_publication_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        command_digests: Vec<CommandCaptureDigest>,
        _settled_outcome_sha256: String,
        completed_at_unix_ms: u64,
        failure: PublicationFailure,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        let mut originating_error_ids = vec![failure.id];
        let ambiguous_outcome_sha256 = match outcome_digest(
            effect_identity_sha256,
            effect,
            HostEffectState::Ambiguous,
            &command_digests,
            self.policy.policy_sha256(),
            completed_at_unix_ms,
        ) {
            Ok(outcome_sha256) => outcome_sha256,
            Err(outcome_failure) => {
                append_error_cause(&mut originating_error_ids, outcome_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    Some(PriorPublicationEvidence {
                        publication_identity_sha256: None,
                        observation: failure.observation.as_ref(),
                    }),
                ));
            }
        };
        let terminal = match self.transition_terminal(
            effect,
            HostEffectState::Ambiguous,
            ambiguous_outcome_sha256,
        ) {
            Ok(terminal) => terminal,
            Err(transition_failure) => {
                append_error_cause(&mut originating_error_ids, transition_failure.id());
                return Err(self.publication_before_terminal_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    &failure,
                    originating_error_ids,
                ));
            }
        };
        let (observation, classification) = match self
            .target
            .reobserve_failure(&failure, terminal.current_head().clone())
        {
            Ok(current) => current,
            Err(observation_failure) => {
                append_error_cause(&mut originating_error_ids, observation_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    Some(PriorPublicationEvidence {
                        publication_identity_sha256: None,
                        observation: failure.observation.as_ref(),
                    }),
                ));
            }
        };
        let recovery = HostEffectRecoveryHandoff::publication(
            effect_identity_sha256.to_owned(),
            effect.permit().permit_id().to_owned(),
            terminal.current_head().clone(),
            observation,
            classification,
            originating_error_ids,
        );
        debug_assert!(recovery.verify_binding());
        Err(HostEffectExecutorFailure::with_recovery(
            failure.id,
            Some(HostEffectState::Ambiguous),
            recovery,
        ))
    }

    fn committed_recovery_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        committed: &CommittedPublication,
        ledger_head: HostEffectLedgerHead,
        id: HostEffectExecutorErrorId,
    ) -> HostEffectExecutorFailure {
        match self
            .target
            .reobserve_committed(committed, ledger_head.clone())
        {
            Ok((observation, classification)) => {
                let recovery = HostEffectRecoveryHandoff::publication(
                    effect_identity_sha256.to_owned(),
                    effect.permit().permit_id().to_owned(),
                    ledger_head,
                    observation,
                    classification,
                    vec![id],
                );
                debug_assert!(recovery.verify_binding());
                HostEffectExecutorFailure::with_recovery(
                    id,
                    Some(HostEffectState::Settled),
                    recovery,
                )
            }
            Err(observation_failure) => self.post_reservation_recovery_failure(
                effect,
                effect_identity_sha256,
                id,
                vec![id, observation_failure.id()],
                Some(PriorPublicationEvidence {
                    publication_identity_sha256: Some(
                        committed.expectation.publication_identity_sha256(),
                    ),
                    observation: Some(&committed.observation),
                }),
            ),
        }
    }

    fn committed_before_terminal_recovery_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        committed: &CommittedPublication,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
    ) -> HostEffectExecutorFailure {
        let mut originating_error_ids = originating_error_ids;
        let ledger_head = match self.ledger.head() {
            Ok(ledger_head) => ledger_head,
            Err(_ledger_observation_failure) => {
                append_error_cause(
                    &mut originating_error_ids,
                    HostEffectExecutorErrorId::LedgerSubstitution,
                );
                effect.record().current_head().clone()
            }
        };
        match self
            .target
            .reobserve_committed(committed, ledger_head.clone())
        {
            Ok((observation, classification)) => {
                let recovery = HostEffectRecoveryHandoff::publication(
                    effect_identity_sha256.to_owned(),
                    effect.permit().permit_id().to_owned(),
                    ledger_head,
                    observation,
                    classification,
                    originating_error_ids,
                );
                debug_assert!(recovery.verify_binding());
                HostEffectExecutorFailure::with_recovery(
                    HostEffectExecutorErrorId::RecoveryRequired,
                    None,
                    recovery,
                )
            }
            Err(observation_failure) => {
                append_error_cause(&mut originating_error_ids, observation_failure.id());
                self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    Some(PriorPublicationEvidence {
                        publication_identity_sha256: Some(
                            committed.expectation.publication_identity_sha256(),
                        ),
                        observation: Some(&committed.observation),
                    }),
                )
            }
        }
    }

    fn publication_before_terminal_recovery_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        failure: &PublicationFailure,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
    ) -> HostEffectExecutorFailure {
        let mut originating_error_ids = originating_error_ids;
        let ledger_head = match self.ledger.head() {
            Ok(ledger_head) => ledger_head,
            Err(_ledger_observation_failure) => {
                append_error_cause(
                    &mut originating_error_ids,
                    HostEffectExecutorErrorId::LedgerSubstitution,
                );
                effect.record().current_head().clone()
            }
        };
        match self.target.reobserve_failure(failure, ledger_head.clone()) {
            Ok((observation, classification)) => {
                let recovery = HostEffectRecoveryHandoff::publication(
                    effect_identity_sha256.to_owned(),
                    effect.permit().permit_id().to_owned(),
                    ledger_head,
                    observation,
                    classification,
                    originating_error_ids,
                );
                debug_assert!(recovery.verify_binding());
                HostEffectExecutorFailure::with_recovery(
                    HostEffectExecutorErrorId::RecoveryRequired,
                    None,
                    recovery,
                )
            }
            Err(observation_failure) => {
                append_error_cause(&mut originating_error_ids, observation_failure.id());
                self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    Some(PriorPublicationEvidence {
                        publication_identity_sha256: None,
                        observation: failure.observation.as_ref(),
                    }),
                )
            }
        }
    }

    fn post_reservation_recovery_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        returned_error_id: HostEffectExecutorErrorId,
        mut originating_error_ids: Vec<HostEffectExecutorErrorId>,
        publication: Option<PriorPublicationEvidence<'_>>,
    ) -> HostEffectExecutorFailure {
        let ledger = self.reobserve_post_reservation(effect);
        if let Some(observation_error_id) = ledger.observation_error_id {
            append_error_cause(&mut originating_error_ids, observation_error_id);
        }
        let (
            publication_identity_sha256,
            prior_publication_observation,
            publication_classification,
        ) = match publication {
            Some(evidence) => {
                let classification = if evidence.observation.is_some() {
                    HostEffectPostReservationPublicationClassification::PriorObservationCurrentObservationUnavailable
                } else if evidence.publication_identity_sha256.is_some() {
                    HostEffectPostReservationPublicationClassification::PublicationIdentityOnlyCurrentObservationUnavailable
                } else {
                    HostEffectPostReservationPublicationClassification::PublicationEvidenceUnavailable
                };
                (
                    evidence.publication_identity_sha256.map(str::to_owned),
                    evidence.observation.cloned(),
                    classification,
                )
            }
            None => (
                None,
                None,
                HostEffectPostReservationPublicationClassification::NoPublicationEvidence,
            ),
        };
        let recovery = HostEffectRecoveryHandoff::post_reservation(
            effect_identity_sha256.to_owned(),
            effect.permit().permit_id().to_owned(),
            effect.record().current_head().clone(),
            effect.record().clone(),
            ledger.ledger_head,
            ledger.ledger_record,
            ledger.exact_current_observation,
            publication_identity_sha256,
            prior_publication_observation,
            originating_error_ids,
            ledger.classification,
            publication_classification,
        );
        debug_assert!(recovery.verify_binding());
        HostEffectExecutorFailure::with_recovery(returned_error_id, ledger.terminal_state, recovery)
    }

    fn reobserve_post_reservation(
        &self,
        effect: &AuthorizedHostEffect,
    ) -> PostReservationLedgerEvidence {
        const MAX_STABLE_OBSERVATION_ATTEMPTS: usize = 3;
        let mut observation_failed = false;
        for _ in 0..MAX_STABLE_OBSERVATION_ATTEMPTS {
            let head_before = match self.ledger.head() {
                Ok(head) => head,
                Err(_ledger_observation_failure) => {
                    observation_failed = true;
                    continue;
                }
            };
            let record_before = match self.ledger.read(effect.permit().permit_id()) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(_ledger_observation_failure) => {
                    observation_failed = true;
                    continue;
                }
            };
            let record_after = match self.ledger.read(effect.permit().permit_id()) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(_ledger_observation_failure) => {
                    observation_failed = true;
                    continue;
                }
            };
            let head_after = match self.ledger.head() {
                Ok(head) => head,
                Err(_ledger_observation_failure) => {
                    observation_failed = true;
                    continue;
                }
            };
            if head_before != head_after || record_before != record_after {
                continue;
            }
            if head_after != *record_after.current_head()
                || record_after.reservation().permit_id() != effect.permit().permit_id()
            {
                continue;
            }
            if record_after == *effect.record() && head_after == *record_after.current_head() {
                return PostReservationLedgerEvidence {
                    ledger_head: head_after,
                    ledger_record: record_after,
                    exact_current_observation: true,
                    classification: HostEffectPostReservationLedgerClassification::StillInFlight,
                    terminal_state: None,
                    observation_error_id: None,
                };
            }
            if record_after.reservation() == effect.record().reservation()
                && head_after == *record_after.current_head()
                && matches!(
                    record_after.state(),
                    HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
                )
            {
                let terminal_state = Some(record_after.state());
                return PostReservationLedgerEvidence {
                    ledger_head: head_after,
                    ledger_record: record_after,
                    exact_current_observation: true,
                    classification: HostEffectPostReservationLedgerClassification::TerminalObserved,
                    terminal_state,
                    observation_error_id: None,
                };
            }
            return PostReservationLedgerEvidence {
                ledger_head: head_after,
                ledger_record: record_after,
                exact_current_observation: true,
                classification: HostEffectPostReservationLedgerClassification::ObservationRejected,
                terminal_state: None,
                observation_error_id: None,
            };
        }
        PostReservationLedgerEvidence {
            ledger_head: effect.record().current_head().clone(),
            ledger_record: effect.record().clone(),
            exact_current_observation: false,
            classification: HostEffectPostReservationLedgerClassification::ObservationUnavailable,
            terminal_state: None,
            observation_error_id: observation_failed
                .then_some(HostEffectExecutorErrorId::LedgerSubstitution),
        }
    }

    #[cfg(test)]
    pub(super) fn execute_authorized_for_test(
        &mut self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
        target_lease: &mut dyn HostTargetLease,
        clock: &mut dyn RootTrustedClock,
        cancellation: &HostEffectCancellation,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        self.execute_authorized(capability, effect, target_lease, clock, cancellation)
    }
}

fn receipt_name(permit_id: &str) -> Result<String, HostEffectExecutorFailure> {
    let digest = permit_id
        .strip_prefix("sha256:")
        .ok_or_else(|| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Replay))?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HostEffectExecutorFailure::new(
            HostEffectExecutorErrorId::Replay,
        ));
    }
    Ok(format!("effect-{digest}.json"))
}

fn fallback_effect_identity(effect: &AuthorizedHostEffect) -> String {
    model::digest_bytes(
        format!(
            "harness-ultragoal.unavailable-effect-identity.v1\n{}\n{}",
            effect.permit().permit_id(),
            effect.record().current_head().head_sha256(),
        )
        .as_bytes(),
    )
}

fn append_error_cause(
    originating_error_ids: &mut Vec<HostEffectExecutorErrorId>,
    error_id: HostEffectExecutorErrorId,
) {
    if error_id != HostEffectExecutorErrorId::RecoveryRequired
        && !originating_error_ids.contains(&error_id)
    {
        originating_error_ids.push(error_id);
    }
}

fn terminal_outcome(
    effect: &AuthorizedHostEffect,
    state: HostEffectState,
    command_digests: &[CommandCaptureDigest],
    outcome_sha256: String,
    completed_at_unix_ms: u64,
) -> Result<HostEffectOutcome, HostEffectExecutorFailure> {
    HostEffectOutcome::new(
        effect.permit().permit_id().to_owned(),
        state,
        command_digests
            .iter()
            .map(|capture| capture.combined_sha256.clone())
            .collect(),
        None,
        outcome_sha256,
        completed_at_unix_ms,
    )
    .map_err(|_| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Io))
}

fn outcome_digest(
    effect_identity_sha256: &str,
    effect: &AuthorizedHostEffect,
    state: HostEffectState,
    command_digests: &[CommandCaptureDigest],
    policy_sha256: &str,
    completed_at_unix_ms: u64,
) -> Result<String, HostEffectExecutorFailure> {
    #[derive(Serialize)]
    struct Outcome<'a> {
        schema: &'static str,
        effect_identity_sha256: &'a str,
        permit_id: &'a str,
        state: HostEffectState,
        command_digests: &'a [CommandCaptureDigest],
        policy_sha256: &'a str,
        completed_at_unix_ms: u64,
    }
    digest_json(&Outcome {
        schema: "harness-ultragoal.host-effect-outcome.v1",
        effect_identity_sha256,
        permit_id: effect.permit().permit_id(),
        state,
        command_digests,
        policy_sha256,
        completed_at_unix_ms,
    })
}

fn publication_bytes(
    effect_identity_sha256: &str,
    effect: &AuthorizedHostEffect,
    command_digests: &[CommandCaptureDigest],
    policy_sha256: &str,
    outcome_sha256: &str,
    completed_at_unix_ms: u64,
) -> Result<Vec<u8>, HostEffectExecutorFailure> {
    #[derive(Serialize)]
    struct Publication<'a> {
        schema_version: &'static str,
        effect_identity_sha256: &'a str,
        permit_id: &'a str,
        command_plan_sha256: &'a str,
        command_digests: &'a [CommandCaptureDigest],
        policy_sha256: &'a str,
        outcome_sha256: &'a str,
        completed_at_unix_ms: u64,
    }
    serde_json::to_vec(&Publication {
        schema_version: "SupportedHostEffectPublication-v1",
        effect_identity_sha256,
        permit_id: effect.permit().permit_id(),
        command_plan_sha256: effect.plan().plan_sha256(),
        command_digests,
        policy_sha256,
        outcome_sha256,
        completed_at_unix_ms,
    })
    .map_err(|_| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Io))
}

fn ledger_failure() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::LedgerSubstitution)
}

fn target_failure() -> HostEffectExecutorFailure {
    HostEffectExecutorFailure::new(HostEffectExecutorErrorId::TargetSubstitution)
}

#[cfg(test)]
mod tests;
