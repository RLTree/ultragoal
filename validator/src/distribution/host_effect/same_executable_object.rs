#[cfg(unix)]
fn same_executable_object(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.mode() == right.mode()
        && left.uid() == right.uid()
        && left.gid() == right.gid()
        && left.nlink() == right.nlink()
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(unix)]
fn read_at_retry(
    file: &File,
    buffer: &mut [u8],
    offset: u64,
) -> Result<usize, HostEffectLedgerError> {
    loop {
        match file.read_at(buffer, offset) {
            Ok(count) => return Ok(count),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(ledger_io()),
        }
    }
}

fn invalid_record() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord)
}

fn tampered() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::Tampered)
}

fn ledger_io() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::Io)
}

/// A ledger-reserved effect. Construction remains confined to reviewed
/// host-effect descendants and must occur only after permit verification and a
/// successful Reserved -> InFlight transition.
pub(crate) struct AuthorizedHostEffect {
    permit: HostEffectPermit,
    record: HostEffectLedgerRecord,
    executable: SelectedCodexExecutable,
    plan: HostCommandPlan,
}

impl AuthorizedHostEffect {
    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn new(
        permit: HostEffectPermit,
        record: HostEffectLedgerRecord,
        executable: SelectedCodexExecutable,
        plan: HostCommandPlan,
    ) -> Result<Self, HostEffectLedgerError> {
        Self::validate(&permit, &record, &executable, &plan)?;
        Ok(Self::from_validated(permit, record, executable, plan))
    }

    pub(in crate::distribution::host_effect) fn validate(
        permit: &HostEffectPermit,
        record: &HostEffectLedgerRecord,
        executable: &SelectedCodexExecutable,
        plan: &HostCommandPlan,
    ) -> Result<(), HostEffectLedgerError> {
        executable.revalidate()?;
        let executable_identity_sha256 = executable.binding_sha256()?;
        let expected_reservation = HostEffectReservation::from_permit(&permit);
        if record.state != HostEffectState::InFlight
            || record.reservation != expected_reservation
            || plan.plan_sha256() != permit.binding().command_plan_sha256
            || executable_identity_sha256 != permit.binding().executable_identity_sha256
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(())
    }

    pub(in crate::distribution::host_effect) fn from_validated(
        permit: HostEffectPermit,
        record: HostEffectLedgerRecord,
        executable: SelectedCodexExecutable,
        plan: HostCommandPlan,
    ) -> Self {
        Self {
            permit,
            record,
            executable,
            plan,
        }
    }

    pub(in crate::distribution::host_effect) fn permit(&self) -> &HostEffectPermit {
        &self.permit
    }

    pub(in crate::distribution::host_effect) fn record(&self) -> &HostEffectLedgerRecord {
        &self.record
    }

    pub(in crate::distribution::host_effect) fn executable(&self) -> &SelectedCodexExecutable {
        &self.executable
    }

    pub(in crate::distribution::host_effect) fn plan(&self) -> &HostCommandPlan {
        &self.plan
    }

    pub(in crate::distribution::host_effect) fn finalize(
        self,
    ) -> Result<(), HostEffectLedgerError> {
        self.executable.finalize()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectOutcome {
    permit_id: String,
    state: HostEffectState,
    command_output_sha256: Vec<String>,
    observed_post_state_sha256: Option<String>,
    outcome_sha256: String,
    completed_at_unix_ms: u64,
}

impl HostEffectOutcome {
    pub(in crate::distribution::host_effect) fn new(
        permit_id: String,
        state: HostEffectState,
        command_output_sha256: Vec<String>,
        observed_post_state_sha256: Option<String>,
        outcome_sha256: String,
        completed_at_unix_ms: u64,
    ) -> Result<Self, HostEffectLedgerError> {
        if !is_digest(&permit_id)
            || !matches!(
                state,
                HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
            )
            || command_output_sha256.iter().any(|row| !is_digest(row))
            || observed_post_state_sha256
                .as_ref()
                .is_some_and(|row| !is_digest(row))
            || !is_digest(&outcome_sha256)
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(Self {
            permit_id,
            state,
            command_output_sha256,
            observed_post_state_sha256,
            outcome_sha256,
            completed_at_unix_ms,
        })
    }
}

fn allowed_transition(expected: HostEffectState, next: HostEffectState) -> bool {
    matches!(
        (expected, next),
        (HostEffectState::Reserved, HostEffectState::InFlight)
            | (HostEffectState::Reserved, HostEffectState::Failed)
            | (HostEffectState::InFlight, HostEffectState::Settled)
            | (HostEffectState::InFlight, HostEffectState::Failed)
            | (HostEffectState::InFlight, HostEffectState::Ambiguous)
            | (HostEffectState::Ambiguous, HostEffectState::Settled)
            | (HostEffectState::Ambiguous, HostEffectState::Failed)
    )
}

fn is_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
