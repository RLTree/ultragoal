use super::*;

impl ReservationTransaction {
    pub(super) fn execute_intent(
        &self,
        intent: &super::super::super::super::mediator::IntentExecutionRequest,
    ) -> Result<super::super::super::super::mediator::ProcessObservation, RoutineError> {
        let staged = stage_program(
            &self.launch_root,
            LaunchBinding {
                grant_id: &self.token.grant_id,
                recovery_marker: &self.token.recovery_marker,
            },
            intent.program(),
        )?;
        let stage = bindings::launch_stage_record(&staged, intent);
        self.record_launch_stage(&stage)?;
        let child = RefCell::new(None);
        let observation = catch_unwind(AssertUnwindSafe(|| {
            match super::super::super::super::mediator::prepare_authorized_process(
                &staged.executable,
                intent.root(),
                intent.outputs(),
                intent.reads(),
                intent.argv(),
                intent.environment(),
                intent.framed_input().to_vec(),
                intent.output_budget(),
                intent.cancellation(),
            )? {
                super::super::super::super::mediator::PreparedProcess::Cancelled(observation) => {
                    Ok(observation)
                }
                super::super::super::super::mediator::PreparedProcess::Suspended(process) => {
                    let observed =
                        self.record_started(process.identity()?, &staged.executable, intent)?;
                    *child.borrow_mut() = Some(observed);
                    process.observe(
                        &staged.executable,
                        intent.root(),
                        intent.outputs(),
                        intent.timeout(),
                        intent.cancellation(),
                    )
                }
            }
        }));
        let primary = match observation {
            Ok(Ok(observation)) => catch_unwind(AssertUnwindSafe(|| {
                self.record_reaped(&observation, &child)?;
                Ok(observation)
            })),
            Ok(Err(error)) => catch_unwind(AssertUnwindSafe(|| {
                if child.borrow().is_some()
                    && error
                        .process_custody()
                        .is_some_and(|evidence| evidence.cleanup == CleanupEvidence::Succeeded)
                {
                    self.record_reaped_from_child(&child)?;
                }
                Err(error)
            })),
            Err(payload) => Err(payload),
        };
        let mut cleanup =
            CapturedCleanup::from(catch_unwind(AssertUnwindSafe(|| cleanup_staged(&staged))));
        if cleanup.evidence == CleanupEvidence::Succeeded {
            let recorded = CapturedCleanup::from(catch_unwind(AssertUnwindSafe(|| {
                self.record_launch_cleaned(&stage)
            })));
            if recorded.evidence == CleanupEvidence::Succeeded {
                *self.launch_cleanup.borrow_mut() = CleanupEvidence::Succeeded;
            } else {
                cleanup = recorded;
            }
        }
        finish_execution(primary, cleanup)
    }

    fn record_launch_stage(&self, stage: &store::LaunchStageRecord) -> Result<(), RoutineError> {
        self.resolve(self.durable.ledger.record_launch_stage(
            &mut self.durable.head.borrow_mut(),
            &self.token,
            stage,
        )?)
    }

    fn record_launch_cleaned(&self, stage: &store::LaunchStageRecord) -> Result<(), RoutineError> {
        self.resolve(self.durable.ledger.record_launch_cleaned(
            &mut self.durable.head.borrow_mut(),
            &self.token,
            stage,
        )?)
    }

    fn record_started(
        &self,
        started: super::super::super::super::mediator::StartedProcessIdentity,
        executable: &super::super::super::super::mediator::PinnedExecutable,
        intent: &super::super::super::super::mediator::IntentExecutionRequest,
    ) -> Result<ChildLease, RoutineError> {
        let child = ChildLease {
            process_id: started.process_id(),
            process_group_id: started.process_group_id(),
            executable_sha256: executable.sha256.clone(),
            executable_device: executable.identity.device,
            executable_inode: executable.identity.inode,
            intent: bindings::observed_intent_binding(intent, &executable.sha256),
        };
        match self.durable.ledger.prepare_spawn(
            &mut self.durable.head.borrow_mut(),
            &self.token,
            child.clone(),
        )? {
            DurableWrite::Committed(()) => {
                self.started.set(true);
                Ok(child)
            }
            DurableWrite::Precommit(()) => {
                Err(error("routine-production-authority-publish-precommit"))
            }
            DurableWrite::Ambiguous(()) => {
                self.started.set(true);
                self.ambiguous.set(true);
                Err(error("routine-production-authority-publish-ambiguous"))
            }
        }
    }

    fn record_reaped(
        &self,
        observation: &super::super::super::super::mediator::ProcessObservation,
        child: &RefCell<Option<ChildLease>>,
    ) -> Result<(), RoutineError> {
        if child.borrow().is_none()
            && observation.termination
                == super::super::super::super::mediator::ProcessTermination::Cancelled
        {
            return Ok(());
        }
        self.record_reaped_from_child(child)
    }

    fn record_reaped_from_child(
        &self,
        child: &RefCell<Option<ChildLease>>,
    ) -> Result<(), RoutineError> {
        let observed = child.borrow().clone();
        match observed.as_ref() {
            Some(observed_child) => {
                *self.process_cleanup.borrow_mut() = CleanupEvidence::Succeeded;
                self.resolve(self.durable.ledger.record_process_reaped(
                    &mut self.durable.head.borrow_mut(),
                    &self.token,
                    observed_child,
                )?)?;
                child.borrow_mut().take();
                Ok(())
            }
            None => Err(error("routine-production-child-start-unobserved")),
        }
    }
}
