impl FixtureScheduler {
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
