use super::*;

impl FileAuthorityLedger {
    pub(crate) fn open_or_initialize(root: &Path) -> Result<(Self, LocalHead), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return supported::FileLedger::open_or_initialize(root)
                .map(|(inner, head)| (Self { inner }, head));
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = root;
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn reserve(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
        owner: OwnerLease,
    ) -> Result<DurableWrite<u64>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.reserve(head, token, owner);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (head, token, owner);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn validate_reserved(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
    ) -> Result<(), RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.validate_reserved(head, token);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (head, token);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn prepare_spawn(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
        child: ChildLease,
    ) -> Result<DurableWrite<()>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.prepare_spawn(head, token, &child);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (head, token, child);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn record_launch_stage(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
        stage: &LaunchStageRecord,
    ) -> Result<DurableWrite<()>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.record_launch_stage(head, token, stage);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (head, token, stage);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn record_launch_cleaned(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
        stage: &LaunchStageRecord,
    ) -> Result<DurableWrite<()>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.record_launch_cleaned(head, token, stage);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (head, token, stage);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn record_process_reaped(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
        child: &ChildLease,
    ) -> Result<DurableWrite<()>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.record_process_reaped(head, token, child);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (head, token, child);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn record_output_component(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
    ) -> Result<DurableWrite<()>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self
                .inner
                .record_output_component(head, token, relative_path, identity);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (head, token, relative_path, identity);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn record_output_staged(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
        relative_path: &str,
        identity: OutputDirectoryIdentity,
    ) -> Result<DurableWrite<()>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self
                .inner
                .record_output_staged(head, token, relative_path, identity);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (head, token, relative_path, identity);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }

    pub(crate) fn record_output_transition(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
        transition: &super::super::super::output_journal::OutputTransition,
    ) -> Result<DurableWrite<()>, RoutineError> {
        match transition {
            super::super::super::output_journal::OutputTransition::Staged {
                relative_path,
                identity,
            } => self.record_output_staged(head, token, relative_path, *identity),
            super::super::super::output_journal::OutputTransition::Published {
                relative_path,
                identity,
            } => self.record_output_component(head, token, relative_path, *identity),
        }
    }

    pub(crate) fn settle_terminal(
        &self,
        head: &mut LocalHead,
        token: &ReservationToken,
        terminal: TerminalRecord,
    ) -> Result<DurableWrite<()>, RoutineError> {
        #[cfg(target_vendor = "apple")]
        {
            return self.inner.settle_terminal(head, token, terminal);
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (head, token, terminal);
            Err(error("routine-production-authority-host-unsupported"))
        }
    }
}

pub(crate) fn error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
