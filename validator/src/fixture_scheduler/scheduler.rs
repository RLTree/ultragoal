use super::lease::isolated_environment;
use super::{
    ExpectedOutcome, FixtureScheduleError, FixtureSpec, IsolationLease, LeaseDisposition,
    ObservedOutcome,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Crate-internal boundary between scheduling and process confinement.  The
/// scheduler owns lease lifetime and outcome authority; an adapter owns the
/// pinned command and the observation substrate.  It is deliberately not a
/// public callback, so an external caller cannot replace execution with a
/// fabricated `ObservedOutcome`.
pub(crate) trait FixtureExecutor {
    fn execute(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ObservedOutcome, FixtureScheduleError>;
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

impl FixtureScheduler {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            next_ordinal: 0,
            active: BTreeMap::new(),
        }
    }

    pub fn schedule(
        &mut self,
        specs: impl IntoIterator<Item = FixtureSpec>,
    ) -> Result<Vec<String>, FixtureScheduleError> {
        let mut ordered = specs.into_iter().collect::<Vec<_>>();
        ordered.sort_by(|left, right| left.id.cmp(&right.id));
        let mut seen = BTreeSet::new();
        for spec in &ordered {
            spec.validate()?;
            if !seen.insert(spec.id.clone())
                || self.active.values().any(|run| run.fixture.id == spec.id)
            {
                return Err(FixtureScheduleError::Collision(spec.id.clone()));
            }
        }
        let mut ids = Vec::with_capacity(ordered.len());
        for spec in ordered {
            ids.push(self.acquire(spec)?);
        }
        Ok(ids)
    }

    pub fn run(&self, lease_id: &str) -> Result<&FixtureRun, FixtureScheduleError> {
        self.active
            .get(lease_id)
            .ok_or_else(|| FixtureScheduleError::UnknownLease(lease_id.to_owned()))
    }

    pub fn finish(
        &mut self,
        lease_id: &str,
        _observed: ObservedOutcome,
    ) -> Result<RunDisposition, FixtureScheduleError> {
        let mut run = self
            .active
            .remove(lease_id)
            .ok_or_else(|| FixtureScheduleError::UnknownLease(lease_id.to_owned()))?;
        // Observations supplied by a caller are evidence-free metadata.  They
        // used to mint Accepted/CausalFailure outcomes without a process ever
        // running.  Keep the method only as a cleanup-safe migration boundary;
        // execution-capable adapters must derive their observation themselves.
        if let Err(error) = run.lease.cleanup() {
            return self.retain_for_recovery(lease_id, run, error);
        }
        Err(FixtureScheduleError::Integrity(
            "caller-supplied ObservedOutcome is non-authoritative".to_owned(),
        ))
    }

    pub(crate) fn finish_observed(
        &mut self,
        lease_id: &str,
        observed: ObservedOutcome,
    ) -> Result<RunDisposition, FixtureScheduleError> {
        let mut run = self
            .active
            .remove(lease_id)
            .ok_or_else(|| FixtureScheduleError::UnknownLease(lease_id.to_owned()))?;
        let disposition = if run.fixture.flaky {
            if let Err(error) = run.lease.cleanup() {
                return self.retain_for_recovery(lease_id, run, error);
            }
            RunDisposition::Quarantined
        } else {
            let expected = run.fixture.expected.clone();
            let matched = observed.matches(&expected);
            if let Err(error) = run.lease.cleanup() {
                return self.retain_for_recovery(lease_id, run, error);
            }
            matched?;
            if expected.verdict == super::OutcomeVerdict::Pass {
                RunDisposition::Accepted
            } else {
                RunDisposition::CausalFailure
            }
        };
        Ok(disposition)
    }

    pub(crate) fn execute<E: FixtureExecutor>(
        &mut self,
        lease_id: &str,
        executor: &E,
    ) -> Result<RunDisposition, FixtureScheduleError> {
        let (fixture, lease, environment) = {
            let run = self
                .active
                .get(lease_id)
                .ok_or_else(|| FixtureScheduleError::UnknownLease(lease_id.to_owned()))?;
            (
                run.fixture.clone(),
                run.lease.root().to_path_buf(),
                run.environment.clone(),
            )
        };
        let observed = {
            let run = self
                .active
                .get(lease_id)
                .ok_or_else(|| FixtureScheduleError::UnknownLease(lease_id.to_owned()))?;
            executor.execute(&fixture, &run.lease, &environment)?
        };
        // Bind the adapter result to the immutable fixture after execution,
        // before any cleanup can discard the evidence substrate.
        fixture.validate()?;
        if lease
            != self
                .active
                .get(lease_id)
                .ok_or_else(|| FixtureScheduleError::UnknownLease(lease_id.to_owned()))?
                .lease
                .root()
        {
            return Err(FixtureScheduleError::Integrity(
                "fixture lease changed during execution".to_owned(),
            ));
        }
        self.finish_observed(lease_id, observed)
    }

    pub fn recover(&mut self, lease_id: &str) -> Result<RunDisposition, FixtureScheduleError> {
        let mut run = self
            .active
            .remove(lease_id)
            .ok_or_else(|| FixtureScheduleError::UnknownLease(lease_id.to_owned()))?;
        if let Err(error) = run.lease.recover() {
            return self.retain_for_recovery(lease_id, run, error);
        }
        Ok(RunDisposition::CausalFailure)
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    fn acquire(&mut self, spec: FixtureSpec) -> Result<String, FixtureScheduleError> {
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        let lease = IsolationLease::acquire(&self.root, &spec, self.next_ordinal)?;
        let environment = isolated_environment(&lease);
        let lease_id = lease.id().to_owned();
        let run = FixtureRun {
            fixture: spec,
            lease,
            environment,
        };
        self.active.insert(lease_id.clone(), run);
        Ok(lease_id)
    }

    fn retain_for_recovery(
        &mut self,
        lease_id: &str,
        run: FixtureRun,
        error: FixtureScheduleError,
    ) -> Result<RunDisposition, FixtureScheduleError> {
        match error {
            FixtureScheduleError::Cleanup { .. }
                if run.lease.disposition() == &LeaseDisposition::RecoveryRequired =>
            {
                self.active.insert(lease_id.to_owned(), run);
                Ok(RunDisposition::CleanupFailure)
            }
            FixtureScheduleError::Cleanup { .. } => Err(FixtureScheduleError::Integrity(
                "cleanup failure did not transition the lease to recovery-required".to_owned(),
            )),
            other => Err(other),
        }
    }
}

impl FixtureRun {
    pub fn is_active(&self) -> bool {
        self.lease.disposition() == &LeaseDisposition::Active
    }
    pub fn expected(&self) -> &ExpectedOutcome {
        &self.fixture.expected
    }
}
