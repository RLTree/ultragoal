/// Crate-internal boundary between scheduling and process confinement.  The
/// scheduler owns lease lifetime and outcome authority; an adapter owns the
/// pinned command and the observation substrate.  It is deliberately not a
/// public callback, so an external caller cannot replace execution with a
/// fabricated `ObservedOutcome`.
#[cfg(test)]
pub(crate) trait FixtureExecutor {
    fn execute(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ObservedOutcome, FixtureScheduleError>;
}

/// Narrow extension used only when the crate must retain a bounded execution
/// record before cleanup. Keeping this separate preserves the established
/// executor contract for scheduler callers that need only a derived outcome.
pub(crate) trait RecordedFixtureExecutor {
    fn execute_recorded(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ExecutedFixture, FixtureScheduleError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunDisposition {
    Accepted,
    CausalFailure,
    Quarantined,
    CleanupFailure,
}

#[derive(Debug)]
pub struct FixtureRun {
    pub fixture: FixtureSpec,
    pub lease: IsolationLease,
    pub environment: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct FixtureScheduler {
    root: PathBuf,
    next_ordinal: u64,
    active: BTreeMap<String, FixtureRun>,
}
