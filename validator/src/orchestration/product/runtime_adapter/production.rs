use super::*;
use crate::orchestration::product::command::RootActionRequest;
use crate::orchestration::product::{
    journal_head_identity, open_engine, PermitReplayState, ProductError, ProductionRootAuthority,
    ReadOnlySink, ReconcileOutcome, ReconcileRequest, ReservationObservation, RootPermit,
};
use crate::orchestration::{
    encode_orchestration_log, orchestration_head_for, EffectResolution, EventLog, FileJournal,
    OrchestrationEvent,
};

impl OrchestrationRuntimeAdapter<'_> {
    pub fn issue_production_action(
        &self,
        authority: &ProductionRootAuthority,
        source: RuntimeActionSource<'_>,
        action: &RootActionRequest,
        expires_tick: u64,
    ) -> Result<RootPermit, ProductError> {
        self.with_journal_lock(|| {
            let issued_tick = match source {
                RuntimeActionSource::Current(view) => {
                    view.revalidate_exact(self.context, self.workspace)?;
                    view.state().validate_action(self.workspace, action)?;
                    view.inspection_request().tick
                }
                RuntimeActionSource::Interrupted(view) => {
                    view.revalidate_exact(self.context, self.workspace)?;
                    view.preview().validate_action(self.workspace, action)?;
                    view.inspection_request().tick
                }
            };
            authority.issue_validated_action(action, issued_tick, expires_tick)
        })
    }

    pub fn issue_production_reconcile(
        &self,
        authority: &ProductionRootAuthority,
        view: &CurrentRuntimeView,
        action: &RootActionRequest,
        expires_tick: u64,
        resolution: &EffectResolution,
    ) -> Result<RootPermit, ProductError> {
        self.with_journal_lock(|| {
            view.revalidate_exact(self.context, self.workspace)?;
            view.state().validate_action(self.workspace, action)?;
            authority.issue_validated_reconcile(
                action,
                view.inspection_request().tick,
                expires_tick,
                resolution,
            )
        })
    }

    pub fn execute_production_action(
        &self,
        authority: &ProductionRootAuthority,
        source: RuntimeActionSource<'_>,
        action: &RootActionRequest,
        permit: &RootPermit,
        request: &RuntimeActionRequest,
    ) -> Result<RuntimeActionOutcome, ProductError> {
        authority.require_issued_permit(permit)?;
        let reservation = self.with_journal_lock(|| {
            let execution = self.prevalidate_action(source, action, authority, permit, request)?;
            authority.reserve_validated(execution)
        })?;
        authority.complete_action(reservation, || {
            self.execute_action(source, action, &authority.authority, permit, request)
        })
    }

    pub fn execute_production_reconcile(
        &self,
        authority: &ProductionRootAuthority,
        view: &CurrentRuntimeView,
        action: &RootActionRequest,
        permit: &RootPermit,
        request: &ReconcileRequest,
    ) -> Result<ReconcileOutcome, ProductError> {
        authority.require_issued_permit(permit)?;
        let reservation = self.with_journal_lock(|| {
            let execution = self.prevalidate_reconcile(view, action, authority, permit, request)?;
            authority.reserve_validated(execution)
        })?;
        authority.complete_reconcile(reservation, || {
            self.execute_reconcile(view, action, &authority.authority, permit, request)
        })
    }

    pub fn reconcile_production_reservation(
        &self,
        authority: &ProductionRootAuthority,
        view: &CurrentRuntimeView,
        permit: &RootPermit,
    ) -> Result<PermitReplayState, ProductError> {
        self.with_journal_lock(|| {
            view.revalidate_exact(self.context, self.workspace)?;
            let snapshot = &view.state().snapshot;
            let current_identity = journal_head_identity(&snapshot.journal_head)?;
            let engine = open_engine(
                self.context,
                self.workspace,
                &snapshot.journal_head,
                ReadOnlySink,
            )?;
            let (prior_identity, last_event) = terminal_observation(engine.events())?;
            authority.reconcile_observation(
                permit,
                ReservationObservation {
                    expected_root: self.context.root(),
                    binding: self.context.binding(),
                    workspace_identity: self.workspace.identity(),
                    current_identity: &current_identity,
                    snapshot,
                    prior_identity: prior_identity.as_deref(),
                    last_event: last_event.as_ref(),
                },
            )
        })
    }

    pub(crate) fn with_journal_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ProductError>,
    ) -> Result<T, ProductError> {
        self.workspace.verify()?;
        FileJournal::with_existing_exclusive_lock(self.workspace.root(), || {
            self.workspace.verify()?;
            operation()
        })
    }
}

fn terminal_observation(
    events: &[OrchestrationEvent],
) -> Result<(Option<String>, Option<OrchestrationEvent>), ProductError> {
    let Some((last, prior)) = events.split_last() else {
        return Ok((None, None));
    };
    if prior.is_empty() {
        return Ok((None, Some(last.clone())));
    }
    let log = EventLog(prior.to_vec());
    let bytes = encode_orchestration_log(&log).map_err(ProductError::from)?;
    let head = orchestration_head_for(&last.binding, &log, &bytes).map_err(ProductError::from)?;
    Ok((Some(journal_head_identity(&head)?), Some(last.clone())))
}
