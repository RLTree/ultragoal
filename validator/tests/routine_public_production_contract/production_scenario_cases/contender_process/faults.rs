use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use super::custody::group_exists;

#[derive(Default)]
pub(crate) struct TerminationFaults {
    primary_kill_refusals: usize,
    group_signal_refusals: usize,
    reap_status_refusals: usize,
    pipe_drain_refusals: usize,
    observations: Arc<FaultObservations>,
}

#[derive(Default)]
pub(crate) struct FaultObservations {
    primary_kill_refusals: AtomicUsize,
    group_signal_refusals: AtomicUsize,
    reap_status_refusals: AtomicUsize,
    pipe_drain_refusals: AtomicUsize,
    process_group: AtomicI32,
}

impl TerminationFaults {
    pub(crate) fn injected(
        primary_kill_refusals: usize,
        group_signal_refusals: usize,
        reap_status_refusals: usize,
        pipe_drain_refusals: usize,
    ) -> (Self, Arc<FaultObservations>) {
        let observations = Arc::new(FaultObservations::default());
        (
            Self {
                primary_kill_refusals,
                group_signal_refusals,
                reap_status_refusals,
                pipe_drain_refusals,
                observations: Arc::clone(&observations),
            },
            observations,
        )
    }

    pub(super) fn refuse_primary_kill(&mut self) -> bool {
        refuse(
            &mut self.primary_kill_refusals,
            &self.observations.primary_kill_refusals,
        )
    }

    pub(super) fn refuse_group_signal(&mut self) -> bool {
        refuse(
            &mut self.group_signal_refusals,
            &self.observations.group_signal_refusals,
        )
    }

    pub(super) fn refuse_reap_status(&mut self) -> bool {
        refuse(
            &mut self.reap_status_refusals,
            &self.observations.reap_status_refusals,
        )
    }

    pub(super) fn refuse_pipe_drain(&mut self) -> bool {
        refuse(
            &mut self.pipe_drain_refusals,
            &self.observations.pipe_drain_refusals,
        )
    }

    pub(super) fn observe_process_group(&self, process_group: i32) {
        self.observations
            .process_group
            .store(process_group, Ordering::SeqCst);
    }
}

impl FaultObservations {
    pub(crate) fn primary_kill_refusals(&self) -> usize {
        self.primary_kill_refusals.load(Ordering::SeqCst)
    }

    pub(crate) fn group_signal_refusals(&self) -> usize {
        self.group_signal_refusals.load(Ordering::SeqCst)
    }

    pub(crate) fn reap_status_refusals(&self) -> usize {
        self.reap_status_refusals.load(Ordering::SeqCst)
    }

    pub(crate) fn pipe_drain_refusals(&self) -> usize {
        self.pipe_drain_refusals.load(Ordering::SeqCst)
    }

    pub(crate) fn process_group(&self) -> i32 {
        self.process_group.load(Ordering::SeqCst)
    }

    pub(crate) fn group_is_absent(&self) -> bool {
        let process_group = self.process_group();
        process_group > 0 && matches!(group_exists(process_group), Ok(false))
    }
}

fn refuse(remaining: &mut usize, observed: &AtomicUsize) -> bool {
    if *remaining == 0 {
        return false;
    }
    *remaining -= 1;
    observed.fetch_add(1, Ordering::SeqCst);
    true
}
