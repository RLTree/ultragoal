use crate::fixture_scheduler::LeaseAcquisitionFailure;
#[cfg(all(test, unix))]
use crate::fixture_scheduler::{LeaseAcquisitionStage, run_lease_acquisition_hook};

impl FixtureScheduler {
    fn acquire(&mut self, spec: FixtureSpec) -> Result<(String, FixtureRun), FixtureScheduleError> {
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        let lease = match IsolationLease::acquire(&self.root, &spec, self.next_ordinal) {
            Ok(lease) => lease,
            Err(LeaseAcquisitionFailure::Failed(source)) => return Err(source),
            Err(LeaseAcquisitionFailure::RecoveryRequired { lease, source }) => {
                let lease_id = lease.id().to_owned();
                self.active.insert(
                    lease_id.clone(),
                    FixtureRun {
                        fixture: spec,
                        environment: isolated_environment(&lease),
                        lease,
                    },
                );
                return Err(FixtureScheduleError::schedule_rollback(source, vec![lease_id]));
            }
        };
        let environment = isolated_environment(&lease);
        let lease_id = lease.id().to_owned();
        let run = FixtureRun {
            fixture: spec,
            lease,
            environment,
        };
        #[cfg(all(test, unix))]
        run_lease_acquisition_hook(LeaseAcquisitionStage::BeforeInsertion, run.lease.root());
        Ok((lease_id, run))
    }

    fn rollback_acquisition(
        &mut self,
        acquired: Vec<(String, FixtureRun)>,
        source: FixtureScheduleError,
    ) -> FixtureScheduleError {
        let (source, mut recovery_lease_ids) = match source {
            FixtureScheduleError::ScheduleRollback {
                source,
                recovery_lease_ids,
            } => (*source, recovery_lease_ids),
            source => (source, Vec::new()),
        };
        for (lease_id, mut run) in acquired {
            match run.lease.cleanup() {
                Ok(()) => {}
                Err(FixtureScheduleError::Cleanup { .. })
                    if run.lease.disposition() == &LeaseDisposition::RecoveryRequired =>
                {
                    recovery_lease_ids.push(lease_id.clone());
                    self.active.insert(lease_id, run);
                }
                Err(_) => {
                    // A cleanup error must never discard the only durable
                    // handle for a partially acquired lease.
                    recovery_lease_ids.push(lease_id.clone());
                    self.active.insert(lease_id, run);
                }
            }
        }
        FixtureScheduleError::schedule_rollback(source, recovery_lease_ids)
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
}
