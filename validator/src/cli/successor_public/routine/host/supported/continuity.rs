use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_CHECKPOINT_BYTES: u64 = 16 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContinuationCheckpoint {
    schema_version: String,
    generation: u64,
    target: String,
    context_id: String,
    candidate_id: String,
    plan_id: String,
    snapshot_id: String,
    continuation: String,
    recovery_marker: String,
    attempt_grant: String,
    authenticated_ledger_head: String,
    state: String,
    event_id: String,
    event_observed_at_unix_ms: u64,
    event_sequence: u64,
    event_parent_id: Option<String>,
    event_status: String,
    event_transition: String,
}

impl ContinuationCheckpoint {
    pub(crate) fn ledger_head(&self) -> &str {
        &self.authenticated_ledger_head
    }

    pub(crate) fn attempt_grant(&self) -> &str {
        &self.attempt_grant
    }

    pub(crate) fn is_reserved(&self) -> bool {
        self.state == "reserved"
    }

    pub(crate) fn is_complete(&self) -> bool {
        matches!(
            self.state.as_str(),
            "complete-event-pending" | "complete-event-joined"
        )
    }

    pub(crate) fn state(&self) -> &str {
        &self.state
    }

    pub(crate) fn continuation(&self) -> &str {
        &self.continuation
    }
    pub(crate) fn event_id(&self) -> &str {
        &self.event_id
    }
    pub(crate) fn event_observed_at_unix_ms(&self) -> u64 {
        self.event_observed_at_unix_ms
    }
    pub(crate) fn event_sequence(&self) -> u64 {
        self.event_sequence
    }
    pub(crate) fn event_parent_id(&self) -> Option<&str> {
        self.event_parent_id.as_deref()
    }
    pub(crate) fn event_status(&self) -> &str {
        &self.event_status
    }
    pub(crate) fn event_transition(&self) -> &str {
        &self.event_transition
    }
}

impl HostState {
    pub(crate) fn record_reserved_checkpoint(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: &str,
        recovery_marker: &str,
        attempt_grant: &str,
        authenticated_ledger_head: &str,
    ) -> Result<(), HostFailure> {
        if !target.is_absolute()
            || target.to_str().is_none()
            || !continuation.starts_with("routine-cont-")
            || recovery_marker.is_empty()
            || attempt_grant.is_empty()
            || authenticated_ledger_head.is_empty()
        {
            return Err(HostFailure::Invalid);
        }
        if self.read_optional_checkpoint()?.is_some() {
            return Err(HostFailure::Busy);
        }
        let checkpoint = checkpoint(
            target,
            context_id,
            candidate_id,
            plan_id,
            snapshot_id,
            continuation,
            recovery_marker,
            attempt_grant,
            authenticated_ledger_head,
            "reserved",
            1,
        )?;
        write_checkpoint(&self.adapter, &checkpoint, false)?;
        self.verify()
    }

    pub(crate) fn record_complete_checkpoint(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: &str,
        attempt_grant: &str,
        authenticated_ledger_head: &str,
    ) -> Result<(), HostFailure> {
        let previous = self.read_optional_checkpoint()?;
        if let Some(previous) = &previous {
            validate_checkpoint(
                previous,
                target,
                context_id,
                candidate_id,
                plan_id,
                snapshot_id,
                None,
            )?;
            if previous.state != "reconciled" {
                return Err(HostFailure::Busy);
            }
        }
        let generation = match &previous {
            Some(value) => value
                .generation
                .checked_add(1)
                .ok_or(HostFailure::Invalid)?,
            None => 1,
        };
        let next = checkpoint(
            target,
            context_id,
            candidate_id,
            plan_id,
            snapshot_id,
            continuation,
            "",
            attempt_grant,
            authenticated_ledger_head,
            "complete-event-pending",
            generation,
        )?;
        write_checkpoint(&self.adapter, &next, previous.is_some())?;
        self.verify()
    }

    pub(crate) fn mark_event_joined(
        &self,
        checkpoint: &ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        let current = self
            .exact_checkpoint(
                Path::new(&checkpoint.target),
                &checkpoint.context_id,
                &checkpoint.candidate_id,
                &checkpoint.plan_id,
                &checkpoint.snapshot_id,
                Some(&checkpoint.continuation),
            )?
            .ok_or(HostFailure::Invalid)?;
        if current.state != "complete-event-pending"
            || current.generation != checkpoint.generation
            || current.event_id != checkpoint.event_id
        {
            return Err(HostFailure::Busy);
        }
        let mut next = current;
        next.state = "complete-event-joined".to_owned();
        next.generation = next.generation.checked_add(1).ok_or(HostFailure::Invalid)?;
        write_checkpoint(&self.adapter, &next, true)?;
        self.verify()
    }

    pub(crate) fn exact_checkpoint(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: Option<&str>,
    ) -> Result<Option<ContinuationCheckpoint>, HostFailure> {
        let Some(checkpoint) = self.read_optional_checkpoint()? else {
            return Ok(None);
        };
        validate_checkpoint(
            &checkpoint,
            target,
            context_id,
            candidate_id,
            plan_id,
            snapshot_id,
            continuation,
        )?;
        Ok(Some(checkpoint))
    }

    pub(crate) fn mark_checkpoint_reconciled(
        &self,
        checkpoint: &ContinuationCheckpoint,
        authenticated_ledger_head: String,
    ) -> Result<(), HostFailure> {
        let current = self
            .exact_checkpoint(
                Path::new(&checkpoint.target),
                &checkpoint.context_id,
                &checkpoint.candidate_id,
                &checkpoint.plan_id,
                &checkpoint.snapshot_id,
                Some(&checkpoint.continuation),
            )?
            .ok_or(HostFailure::Invalid)?;
        if !current.is_reserved()
            || current.generation != checkpoint.generation
            || current.authenticated_ledger_head != checkpoint.authenticated_ledger_head
        {
            return Err(HostFailure::Busy);
        }
        let mut next = current;
        next.state = "reconciled".to_owned();
        next.authenticated_ledger_head = authenticated_ledger_head;
        next.generation = next.generation.checked_add(1).ok_or(HostFailure::Invalid)?;
        next.event_id = terminal_event_id(&next.continuation, &next.authenticated_ledger_head);
        next.event_sequence = next.generation;
        write_checkpoint(&self.adapter, &next, true)?;
        self.verify()
    }

    fn read_optional_checkpoint(&self) -> Result<Option<ContinuationCheckpoint>, HostFailure> {
        if !self
            .adapter
            .entry_names()?
            .iter()
            .any(|name| name == CONTINUITY_CHECKPOINT_NAME)
        {
            return Ok(None);
        }
        let file = self
            .adapter
            .open_regular(CONTINUITY_CHECKPOINT_NAME, libc::O_RDONLY, 0o600)?;
        let mut bytes = Vec::new();
        file.take(MAX_CHECKPOINT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| HostFailure::Invalid)?;
        if bytes.len() as u64 > MAX_CHECKPOINT_BYTES {
            return Err(HostFailure::Invalid);
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| HostFailure::Invalid)
    }
}

fn checkpoint(
    target: &Path,
    context_id: &str,
    candidate_id: &str,
    plan_id: &str,
    snapshot_id: &str,
    continuation: &str,
    recovery_marker: &str,
    attempt_grant: &str,
    authenticated_ledger_head: &str,
    state: &str,
    generation: u64,
) -> Result<ContinuationCheckpoint, HostFailure> {
    if !target.is_absolute()
        || target.to_str().is_none()
        || !continuation.starts_with("routine-cont-")
        || attempt_grant.is_empty()
        || authenticated_ledger_head.is_empty()
        || generation == 0
    {
        return Err(HostFailure::Invalid);
    }
    Ok(ContinuationCheckpoint {
        schema_version: "RoutineContinuationCheckpoint-v2".to_owned(),
        generation,
        target: target.to_str().ok_or(HostFailure::Invalid)?.to_owned(),
        context_id: context_id.to_owned(),
        candidate_id: candidate_id.to_owned(),
        plan_id: plan_id.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        continuation: continuation.to_owned(),
        recovery_marker: recovery_marker.to_owned(),
        attempt_grant: attempt_grant.to_owned(),
        authenticated_ledger_head: authenticated_ledger_head.to_owned(),
        state: state.to_owned(),
        event_id: terminal_event_id(continuation, authenticated_ledger_head),
        event_observed_at_unix_ms: 0,
        event_sequence: generation,
        event_parent_id: None,
        event_status: if state.starts_with("complete") {
            "pass"
        } else {
            "blocked"
        }
        .to_owned(),
        event_transition: if state.starts_with("complete") {
            "executed"
        } else {
            "interrupted"
        }
        .to_owned(),
    })
}

fn validate_checkpoint(
    checkpoint: &ContinuationCheckpoint,
    target: &Path,
    context_id: &str,
    candidate_id: &str,
    plan_id: &str,
    snapshot_id: &str,
    continuation: Option<&str>,
) -> Result<(), HostFailure> {
    if checkpoint.schema_version != "RoutineContinuationCheckpoint-v2"
        || checkpoint.generation == 0
        || !matches!(
            checkpoint.state.as_str(),
            "reserved" | "reconciled" | "complete-event-pending" | "complete-event-joined"
        )
        || checkpoint.target != target.to_str().ok_or(HostFailure::Invalid)?
        || checkpoint.context_id != context_id
        || checkpoint.candidate_id != candidate_id
        || checkpoint.plan_id != plan_id
        || checkpoint.snapshot_id != snapshot_id
        || !checkpoint.continuation.starts_with("routine-cont-")
        || checkpoint.attempt_grant.is_empty()
        || checkpoint.authenticated_ledger_head.is_empty()
        || checkpoint.event_id
            != terminal_event_id(
                &checkpoint.continuation,
                &checkpoint.authenticated_ledger_head,
            )
        || checkpoint.event_sequence == 0
        || checkpoint.event_status.is_empty()
        || checkpoint.event_transition.is_empty()
        || (checkpoint.state == "reserved" && checkpoint.recovery_marker.is_empty())
        || continuation.is_some_and(|value| checkpoint.continuation != value)
    {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

fn terminal_event_id(continuation: &str, terminal_head: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"routine-terminal-event-v1\0");
    digest.update(continuation.as_bytes());
    digest.update(b"\0");
    digest.update(terminal_head.as_bytes());
    format!("routine-terminal-{:x}", digest.finalize())
}

fn write_checkpoint(
    adapter: &AnchoredDirectory,
    checkpoint: &ContinuationCheckpoint,
    expected_existing: bool,
) -> Result<(), HostFailure> {
    let bytes = serde_json::to_vec(checkpoint).map_err(|_| HostFailure::Invalid)?;
    if bytes.len() as u64 > MAX_CHECKPOINT_BYTES {
        return Err(HostFailure::Invalid);
    }
    adapter.replace_regular_atomically(
        CONTINUITY_CHECKPOINT_NAME,
        CONTINUITY_CHECKPOINT_STAGE_NAME,
        0o600,
        &bytes,
        expected_existing,
    )
}
