impl HostLifecycleCustody {
    pub(crate) fn commit_release(&mut self) -> Result<(), LifecycleError> {
        if self.custody_phase != 0 {
            return Err(LifecycleError::ReplayedPlan);
        }
        self.plan.authorization_seal.consume_transferred()?;
        self.command_plan
            .take()
            .map(|_| self.custody_phase = 1)
            .ok_or(LifecycleError::ReplayedPlan)
    }
}
